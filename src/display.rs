#[derive(Debug)]
pub struct Display {
    video: [u32; 64 * 32],
}

impl Display {
    pub fn new() -> Self {
        Self {
            video: [0; 64 * 32],
        }
    }

    pub fn draw(&mut self, vx: u8, vy: u8, bytes: &[u8]) {
        todo!()
    }

    pub fn clear(&mut self) {
        self.video.iter_mut().for_each(|i| *i = 0);
    }
}
