// src/engine/base_tiles.rs
use std::fs;
use std::path::Path;
use crate::engine::decompressor;

// РАСШИРЕННЫЙ VRAM: 1 Мегабайт (1_048_576 байт).
pub const VRAM_SIZE_BYTES: usize = 1_048_576;

// A Genesis 4bpp tile is 8x8 pixels at 4 bits per pixel = 32 bytes.
pub const TILE_SIZE_BYTES: usize = 32;

// === КАРТА ПАМЯТИ (VRAM MAP) ===
// ВАЖНО: Всё, что связано с интерфейсом (UI), должно лежать ДО 0x10000.

pub const VRAM_BASETILES: usize    = 0x00000; // 8 КБ: Базовые тайлы
pub const VRAM_UI_ICONS: usize     = 0x02000; // 16 КБ: Иконки предметов и магии
pub const VRAM_DIAMOND_MENU: usize = 0x06000; // 8 КБ: Крест-меню (стрелки)
pub const VRAM_PORTRAITS: usize    = 0x08000; // 16 КБ: Портреты персонажей
pub const VRAM_MAP_SPRITES: usize  = 0x0C000; // 16 КБ: Спрайты на карте (герои, NPC)

// Всё, что выше 0x10000 (64 КБ), используется для огромных карт
#[allow(dead_code)]
pub const VRAM_MAP_TILESETS: usize = 0x10000; // 32 КБ: Тайлсеты карт
#[allow(dead_code)]
pub const VRAM_UI_WINDOWS: usize   = 0x20000; // Свободная зона под окна

pub const BASE_TILES_PATH: &str = "src/data/graphics/tech/basetiles.bin";
pub const BASE_TILES_VRAM_ADDRESS: usize = VRAM_BASETILES;

pub const BASE_TILES_DMA_WORDS: usize = 4096;
pub const BASE_TILES_DMA_BYTES: usize = BASE_TILES_DMA_WORDS * 2;

#[derive(Clone, Debug)]
pub struct Vram {
    bytes: Vec<u8>,
}

impl Default for Vram {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl Vram {
    pub fn new() -> Self {
        Self {
            bytes: vec![0; VRAM_SIZE_BYTES],
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn write_bytes(&mut self, address: usize, source: &[u8]) -> Result<(), String> {
        let end = address
            .checked_add(source.len())
            .ok_or_else(|| "VRAM: переполнение адреса при записи".to_string())?;

        if end > self.bytes.len() {
            return Err(format!(
                "VRAM: запись за границы памяти: ${:05X}..${:05X}, размер VRAM=${:05X}",
                address, end, self.bytes.len()
            ));
        }

        self.bytes[address..end].copy_from_slice(source);
        Ok(())
    }

    pub fn read_bytes(&self, address: usize, length: usize) -> Option<&[u8]> {
        let end = address.checked_add(length)?;
        self.bytes.get(address..end)
    }

    pub fn tile(&self, tile_index: usize) -> Option<&[u8]> {
        let address = tile_index.checked_mul(TILE_SIZE_BYTES)?;
        self.read_bytes(address, TILE_SIZE_BYTES)
    }
}

pub fn load_base_tiles() -> Result<Vram, String> {
    load_base_tiles_from(BASE_TILES_PATH)
}

pub fn load_base_tiles_from(path: impl AsRef<Path>) -> Result<Vram, String> {
    let path = path.as_ref();
    let compressed = fs::read(path).map_err(|e| {
        format!("Не удалось прочитать базовые тайлы '{}': {e}", path.display())
    })?;

    let unpacked = decompressor::decompress(&compressed)
        .map_err(|e| format!("Ошибка STACK-распаковки '{}': {e}", path.display()))?;

    if unpacked.len() < BASE_TILES_DMA_BYTES {
        return Err(format!(
            "Базовых тайлов недостаточно: распаковано {} байт, нужно {} байт",
            unpacked.len(), BASE_TILES_DMA_BYTES
        ));
    }

    let mut vram = Vram::new();
    let dma_data = &unpacked[..BASE_TILES_DMA_BYTES];
    vram.write_bytes(BASE_TILES_VRAM_ADDRESS, dma_data)?;

    println!("--- BASE TILES / VRAM ---");
    println!("VRAM size expanded to: {} bytes (1 MB)", vram.len());
    println!("Loaded base tiles: {} bytes", BASE_TILES_DMA_BYTES);
    println!("------------------------");

    Ok(vram)
}