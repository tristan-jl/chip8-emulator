#[derive(Debug)]
pub(crate) struct Lsfr(u16);

impl Lsfr {
    pub fn new() -> Self {
        Self(0x1234)
    }

    fn get(&mut self) -> u8 {
        let bit = (self.0 ^ (self.0 >> 2) ^ (self.0 >> 3) ^ (self.0 >> 5)) & 1;
        self.0 = (self.0 >> 1) | (bit << 15);

        bit as u8
    }

    pub fn gen(&mut self) -> u8 {
        let mut r = 0;
        for i in 0..8 {
            r += self.get() << i;
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut x = Lsfr::new();

        assert_eq!(x.get(), 0);
        assert_eq!(x.get(), 1);
        assert_eq!(x.get(), 1);
        assert_eq!(x.get(), 1);
        assert_eq!(x.get(), 0);
    }

    #[test]
    fn it_works2() {
        let mut x = Lsfr::new();

        assert_eq!(x.gen(), 110);
        assert_eq!(x.gen(), 36);
        assert_eq!(x.gen(), 219);
        assert_eq!(x.gen(), 80);
        assert_eq!(x.gen(), 112);
    }
}
