use std::env;
mod chip8;
mod display;

use crate::chip8::chip8::Chip8;

fn main() {
    let mut chip8 = Chip8::new();
    let args: Vec<String> = env::args().collect();
    let program_location = &args[1];
    chip8.load_from_file(program_location);
    chip8.run();

}