#[derive(Clone, Copy, Debug, Default)]
pub struct Pixel {
    color: f32,
}

impl Pixel {
    pub fn update(&mut self, dt: f64) {
        if self.color > 0.0 {
            self.color -= dt as f32 * 8.0;
            if self.color < 0.0 {
                self.color = 0.0
            }
        }
    }

    pub fn turn_on(&mut self) {
        self.color = 1.0;
    }

    pub fn color_arr(self) -> [f32; 4] {
        [self.color, self.color, self.color, 1.0]
    }
}
