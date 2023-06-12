mod chip_8;
mod constants;

use chip_8::Chip8;

fn main() {
    let mut chip8 = Chip8::build("", 1000, false);
    chip8.cycle();
}
