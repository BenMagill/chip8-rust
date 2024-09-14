pub mod display {

    #[derive(PartialEq)]
    pub enum Pixel {
        Black,
        White,
    }

    pub struct Display {
        pub display: [u64; 32],
    }
    
    impl Display {
        pub fn new() -> Display {
            Display {
                display: [0b0; 32],
            }
        }

        // TODO: draw data, with wrap around
        pub fn draw(x: u64, y: u32, sprite: [u8; 5]) {

        }

        pub fn row(&self, row: u8) -> u64 {
            self.display[row as usize]
        }

        pub fn get_pixel(&self, x: u64, y: u8) -> Pixel {
            let row = self.row(y);
            let offset = ((64-1)-x) as u32;
            // Make a mask for the pixel representing x and apply
            let mut converted = row & 2_u64.pow(offset);

            // Shift the pixel to the rightmost bit
            if converted != 0 {
                converted = converted >> offset;
            }
            
            if converted == 0 {
                Pixel::Black
            } else {
                Pixel::White
            }
        }
    }
}