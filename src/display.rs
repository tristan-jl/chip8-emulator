#[derive(Debug)]
pub(crate) struct Display {
    video: [u32; Self::VIDEO_HEIGHT * Self::VIDEO_WIDTH],
    dirty: bool,
}

impl Display {
    pub(crate) const VIDEO_HEIGHT: usize = 32;
    pub(crate) const VIDEO_WIDTH: usize = 64;
    pub(crate) const SIZE: usize = Self::VIDEO_HEIGHT * Self::VIDEO_WIDTH;

    pub fn new() -> Self {
        Self {
            video: [0; Self::SIZE],
            dirty: true,
        }
    }

    pub fn draw(&mut self, x_pos: usize, y_pos: usize, bytes: &[u8]) -> u8 {
        let mut collision = 0;

        for (j, byte) in bytes.iter().enumerate() {
            for i in 0..8 {
                let x = (x_pos + i) % Self::VIDEO_WIDTH;
                let y = (y_pos + j) % Self::VIDEO_HEIGHT;

                if (byte & (0x80 >> i)) != 0x0 {
                    if self.video[y * Self::VIDEO_WIDTH + x] == 0x1 {
                        collision = 1;
                    }
                    self.video[y * Self::VIDEO_WIDTH + x] ^= 0x1;
                }
            }
        }
        self.dirty = true;

        collision
    }

    pub fn clear(&mut self) {
        self.video.iter_mut().for_each(|i| *i = 0);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_clean(&mut self) {
        self.dirty = false;
    }

    pub fn view(&self) -> &[u32; Self::SIZE] {
        &self.video
    }
}
