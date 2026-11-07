// src/game/map/anim.rs

pub struct MapAnimation {
    pub dest_tileset_idx: usize, // В какой тайлсет карты пишем (обычно 4 - это 5-й)
    pub src_tiles: Vec<u8>,      // Откуда берем кадры (например, 111-й тайлсет)
    pub dest_start_byte: usize,  // С какого байта начинать запись (вычислено из $300 или $350)
    pub len_byte: usize,         // Длина одного кадра в байтах
    pub frame2_offset: usize,    // Смещение для второго кадра (равно len_byte)
    pub timer: u16,
    pub is_frame2: bool,
}

impl MapAnimation {
    pub fn update(&mut self, tilesets: &mut [Vec<u8>]) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
        self.timer = 20; // Задержка между кадрами (можно парсить из файла, пока хардкод)

        // Защита от выхода за границы
        if self.dest_tileset_idx >= tilesets.len() { return; }
        let dest = &mut tilesets[self.dest_tileset_idx];

        // Выбираем кадр (первый или второй)
        let src_offset = if self.is_frame2 { self.frame2_offset } else { 0 };

        if self.dest_start_byte + self.len_byte <= dest.len() && src_offset + self.len_byte <= self.src_tiles.len() {
            // Копируем len_byte байт из src_tiles в dest
            let data_to_copy = self.src_tiles[src_offset..src_offset + self.len_byte].to_vec();
            dest[self.dest_start_byte..self.dest_start_byte + self.len_byte].copy_from_slice(&data_to_copy);
        }

        // Переключаем кадр
        self.is_frame2 = !self.is_frame2;
    }
}