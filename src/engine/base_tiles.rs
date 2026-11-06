// src/engine/base_tiles.rs
use std::fs;
use std::path::Path;
use crate::engine::decompressor;

// Full VRAM size, matching the Genesis VDP's 64 KiB address space.
pub const VRAM_SIZE_BYTES: usize = 0x1_0000;

// A Genesis 4bpp tile is 8x8 pixels at 4 bits per pixel = 32 bytes.
pub const TILE_SIZE_BYTES: usize = 32;

// Default base-tile asset: the shared tile set that occupies VRAM starting at
// $0000. Loaded once at startup and used by every renderer path.
pub const BASE_TILES_PATH: &str = "src/data/graphics/tech/basetiles.bin";
pub const BASE_TILES_VRAM_ADDRESS: usize = 0x0000;

// The original ASM DMA request transfers 4096 words (8192 bytes) from the
// decompressed blob into VRAM, regardless of how much extra data the blob
// contains. Those trailing bytes are ignored on purpose.
pub const BASE_TILES_DMA_WORDS: usize = 4096;
pub const BASE_TILES_DMA_BYTES: usize = BASE_TILES_DMA_WORDS * 2;

/// Byte-addressable VRAM buffer.
///
/// All reads/writes go through checked slicing so an out-of-range access
/// returns an error instead of panicking. Tile access assumes a 32-byte tile
/// size, matching the Genesis 4bpp format.
#[derive(Clone, Debug)]
pub struct Vram {
    bytes: Vec<u8>,
}

impl Default for Vram {
    fn default() -> Self {
        Self::new()
    }
}

// `len()` is only used by diagnostics/logging today; the allow suppresses the
// dead-code warning without hiding a future real use.
#[allow(dead_code)] // Убирает предупреждение, что len() не используется
impl Vram {
    pub fn new() -> Self {
        Self {
            bytes: vec![0; VRAM_SIZE_BYTES],
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Copy `source` into VRAM starting at `address`.
    /// Returns an error if the write would overflow the address space or
    /// exceed the end of VRAM, instead of silently truncating.
    pub fn write_bytes(&mut self, address: usize, source: &[u8]) -> Result<(), String> {
        let end = address
            .checked_add(source.len())
            .ok_or_else(|| "VRAM: переполнение адреса при записи".to_string())?;

        if end > self.bytes.len() {
            return Err(format!(
                "VRAM: запись за границы памяти: ${address:04X}..${end:05X}, размер VRAM=${:05X}",
                self.bytes.len()
            ));
        }

        self.bytes[address..end].copy_from_slice(source);
        Ok(())
    }

    /// Borrow `length` bytes starting at `address`, or `None` if the range
    /// falls outside VRAM.
    pub fn read_bytes(&self, address: usize, length: usize) -> Option<&[u8]> {
        let end = address.checked_add(length)?;
        self.bytes.get(address..end)
    }

    /// Borrow the raw 32-byte tile at `tile_index`, or `None` if the index is
    /// out of range. `tile_index * 32` is computed with checked arithmetic so
    /// a bogus index cannot wrap the address.
    pub fn tile(&self, tile_index: usize) -> Option<&[u8]> {
        let address = tile_index.checked_mul(TILE_SIZE_BYTES)?;
        self.read_bytes(address, TILE_SIZE_BYTES)
    }
}

pub fn load_base_tiles() -> Result<Vram, String> {
    load_base_tiles_from(BASE_TILES_PATH)
}

/// Load `basetiles.bin`, decompress it with the STACK decompressor, and copy
/// the ASM-specified DMA window (8192 bytes from $0000) into a fresh VRAM.
///
/// The decompressed blob may be larger than the DMA window — only the first
/// `BASE_TILES_DMA_BYTES` are written, mirroring what the original 68000 code
/// transferred to the VDP. The final VRAM state is logged for diagnostics.
pub fn load_base_tiles_from(path: impl AsRef<Path>) -> Result<Vram, String> {
    let path = path.as_ref();
    let compressed = fs::read(path).map_err(|e| {
        format!(
            "Не удалось прочитать базовые тайлы '{}': {e}",
            path.display()
        )
    })?;

    // STACK decompression; propagates the specific error from the codec.
    let unpacked = decompressor::decompress(&compressed)
        .map_err(|e| format!("Ошибка STACK-распаковки '{}': {e}", path.display()))?;

    // The DMA window must be fully present; otherwise the VDP would have
    // received fewer bytes than the original game's transfer requested.
    if unpacked.len() < BASE_TILES_DMA_BYTES {
        return Err(format!(
            "Базовых тайлов недостаточно: распаковка дала {} байт, а ASM DMA запрашивает {} байт",
            unpacked.len(),
            BASE_TILES_DMA_BYTES
        ));
    }

    let mut vram = Vram::new();
    // Only the DMA window is uploaded; trailing bytes are intentionally dropped.
    let dma_data = &unpacked[..BASE_TILES_DMA_BYTES];
    vram.write_bytes(BASE_TILES_VRAM_ADDRESS, dma_data)?;

    // Diagnostic dump of the load, useful when comparing against the original
    // game's VRAM state.
    println!("--- BASE TILES / VRAM ---");
    println!("File: {}", path.display());
    println!("Compressed: {} bytes", compressed.len());
    println!("Decompressed: {} bytes", unpacked.len());
    println!(
        "DMA: {} words / {} bytes -> VRAM ${:04X}",
        BASE_TILES_DMA_WORDS,
        BASE_TILES_DMA_BYTES,
        BASE_TILES_VRAM_ADDRESS
    );
    println!("Loaded tiles: {}", BASE_TILES_DMA_BYTES / TILE_SIZE_BYTES);
    println!("VRAM size: {} bytes", vram.len());
    println!("------------------------");

    Ok(vram)
}