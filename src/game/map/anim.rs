// src/game/map/anim.rs

pub struct MapAnimation {
    pub timer: u16,
    pub is_frame2: bool,
}

impl MapAnimation {
    pub fn update(&mut self, tilesets: &mut [Vec<u8>]) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
        self.timer = 20; 

        // Проверки на всякий случай
        if tilesets.len() <= 5 { return; }
        if tilesets[5].len() < 2048 || tilesets[4].len() < 1024 { return; }

        // Если первый кадр - берем первые 1024 байта из 74-го
        // Если второй - берем вторые 1024 байта из 74-го
        let offset = if self.is_frame2 { 1024 } else { 0 };

        // Копируем 1024 байта (32 тайла) во временную коробочку
        let data_to_copy = tilesets[5][offset..offset + 1024].to_vec();

        // И вставляем их в НАЧАЛО 66-го тайлсета (индекс 4)
        tilesets[4][0..1024].copy_from_slice(&data_to_copy);

        self.is_frame2 = !self.is_frame2;
    }
}