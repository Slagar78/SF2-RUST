// src/engine/decompressor.rs

/// MSB-first bit reader over a byte slice.
/// Bits are consumed from bit 7 down to bit 0 of each byte, matching the
/// bit order used by the original 68000 decompression routines.
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, byte_pos: 0, bit_pos: 0 }
    }

    pub fn read_bit(&mut self) -> Result<u8, String> {
        if self.byte_pos >= self.data.len() {
            return Err("Конец данных".to_string());
        }
        let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Ok(bit)
    }

    /// Read `count` bits MSB-first and return them packed into a `u32`.
    pub fn read_bits(&mut self, count: u32) -> Result<u32, String> {
        let mut val = 0u32;
        for _ in 0..count {
            val = (val << 1) | (self.read_bit()? as u32);
        }
        Ok(val)
    }
}

/// Decode one nibble of the "command pattern" table.
/// The prefix code is variable-length; the returned value is the 4-bit
/// nibble stored in the command map for this slot.
fn read_command_pattern(reader: &mut BitReader) -> Result<u8, String> {
    if reader.read_bit()? == 0 { return Ok(0); }
    if reader.read_bit()? == 0 { return Ok(if reader.read_bit()? == 0 { 1 } else { 2 }); }
    if reader.read_bit()? == 0 { return Ok(4); }
    if reader.read_bit()? == 0 { return Ok(8); }
    let val = reader.read_bits(4)?;
    Ok(val as u8)
}

/// Decode one nibble of a "history word" token.
/// Returns an index into the 16-entry move-to-front history table.
fn read_history_index(reader: &mut BitReader) -> Result<usize, String> {
    if reader.read_bit()? == 0 {
        return Ok(if reader.read_bit()? == 0 { 0 } else { 1 });
    }
    if reader.read_bit()? == 0 {
        return Ok(if reader.read_bit()? == 0 { 2 } else { 3 });
    }
    if reader.read_bit()? == 0 {
        return Ok(4);
    }
    if reader.read_bit()? == 0 {
        return Ok(if reader.read_bit()? == 0 { 5 } else { 6 });
    }
    if reader.read_bit()? == 0 {
        return Ok(7);
    }
    if reader.read_bit()? == 0 {
        return Ok(if reader.read_bit()? == 0 { 8 } else { 9 });
    }
    if reader.read_bit()? == 0 {
        return Ok(10);
    }
    if reader.read_bit()? == 0 {
        return Ok(if reader.read_bit()? == 0 { 11 } else { 12 });
    }
    if reader.read_bit()? == 0 {
        return Ok(13);
    }
    Ok(if reader.read_bit()? == 0 { 14 } else { 15 })
}

/// Decode a literal 16-bit word built from four history-nibbles.
/// Each nibble is read through `read_history_index`, which also updates the
/// move-to-front table: the selected entry is hoisted to the front, so
/// frequently-used values drift toward index 0 and cost fewer bits next time.
fn read_history_word(reader: &mut BitReader, history: &mut [u16; 16]) -> Result<u16, String> {
    let mut word = 0u16;
    for _ in 0..4 {
        let idx = read_history_index(reader)?;
        let val = history[idx];
        if idx > 0 {
            history.copy_within(0..idx, 1);
            history[0] = val;
        }
        word = (word << 4) | val;
    }
    Ok(word)
}

/// Back-reference offset, in words (multiply by 2 to get bytes).
fn read_copy_offset(reader: &mut BitReader) -> Result<u16, String> {
    Ok(reader.read_bits(11)? as u16)
}

/// Back-reference length, in words. Encoded in unary-ish form:
/// a terminating `1` ends the run, `01` adds 1, and `00` adds 2.
/// The base value is 2, so the shortest copy is 2 words.
fn read_copy_length(reader: &mut BitReader) -> Result<usize, String> {
    let mut val = 2;
    loop {
        if reader.read_bit()? == 1 { break; }
        if reader.read_bit()? == 1 { val += 1; break; }
        val += 2;
    }
    Ok(val)
}

/// STACK decompression.
///
/// The stream is organised as a sequence of 16-token blocks. Each block
/// begins with a 16-bit "command map" assembled from four 4-bit patterns;
/// the map's bits then select, per token, between a literal history word
/// (bit = 0) and a back-reference (bit = 1). A back-reference with offset 0
/// terminates the stream.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = BitReader::new(data);
    // Move-to-front table for literal nibbles; starts as 0..15 so the first
    // references to common small values cost the fewest bits.
    let mut history: [u16; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let mut out = Vec::new();

    loop {
        // Build the 16-bit command map from four 4-bit patterns.
        let mut command_map = 0u16;
        for _ in 0..4 {
            let pattern = read_command_pattern(&mut reader)?;
            command_map = (command_map << 4) | (pattern as u16);
        }

        // Walk the command map from MSB to LSB, decoding one token per bit.
        for bit_i in (0..16).rev() {
            if (command_map >> bit_i) & 1 == 1 {
                // Back-reference. Offset 0 is the end-of-stream marker.
                let offset = read_copy_offset(&mut reader)?;
                if offset == 0 {
                    return Ok(out);
                }
                let distance = (offset as usize) * 2;
                let length = read_copy_length(&mut reader)?;

                let src_idx = out.len().checked_sub(distance)
                    .ok_or_else(|| "Ошибка ссылки".to_string())?;

                // Copies are word-granular and may overlap the write cursor,
                // so bytes are copied one at a time rather than with a slice
                // copy — this allows run-length style repetition to expand.
                let mut src = src_idx;
                for _ in 0..length {
                    let b1 = out[src];
                    out.push(b1);
                    src += 1;
                    let b2 = out[src];
                    out.push(b2);
                    src += 1;
                }
            } else {
                // Literal token: a 16-bit word decoded through the history table.
                let word = read_history_word(&mut reader, &mut history)?;
                out.push((word >> 8) as u8);
                out.push((word & 0xFF) as u8);
            }
        }
    }
}

/// Basic (a.k.a. "Basic" / SF2 "RLE-like") decompression.
///
/// Unlike STACK, this codec is not bit-oriented: the stream is a sequence
/// of big-endian u16 words. Each 16-word group starts with a command word
/// whose bits are processed MSB-first; each bit consumes one operand word.
///   - bit = 0 : operand is emitted literally.
///   - bit = 1 : operand describes a back-reference; the low 5 bits encode
///               the repeat count, the rest select the source word index.
/// A back-reference operand of 0 terminates the stream.
///
/// This function never fails: truncated or malformed input simply stops
/// output at the last complete token.
pub fn decompress_basic(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos + 2 <= data.len() {
        let cmd = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        for i in (0..16).rev() {
            // Truncated mid-group: stop cleanly.
            if pos + 2 > data.len() { return out; }
            let operand = u16::from_be_bytes([data[pos], data[pos + 1]]);
            pos += 2;
            if (cmd >> i) & 1 != 0 {
                if operand == 0 { return out; }
                let repeats = (operand & 0x1F) as u32;
                let word_idx = (operand - repeats as u16) >> 5;
                let cnt = (33 - repeats) as usize;
                // Special case: word_idx == 1 means "repeat the last emitted word".
                if word_idx == 1 {
                    if out.len() >= 2 {
                        let b1 = out[out.len() - 2];
                        let b2 = out[out.len() - 1];
                        for _ in 0..cnt { out.push(b1); out.push(b2); }
                    }
                } else {
                    // General back-reference: copy `cnt` words from `word_idx`
                    // words back, allowing overlap with the write cursor.
                    let src = (out.len() as isize) - (word_idx as isize) * 2;
                    if src >= 0 {
                        let mut s = src as usize;
                        for _ in 0..cnt {
                            if s + 1 >= out.len() { break; }
                            let b1 = out[s];
                            let b2 = out[s + 1];
                            out.push(b1); out.push(b2);
                            s += 2;
                        }
                    }
                }
            } else {
                // Literal word.
                out.push((operand >> 8) as u8);
                out.push((operand & 0xFF) as u8);
            }
        }
    }
    out
}