mod chip_8;
mod constants;
mod display;

use chip_8::Chip8;

fn main() {
    let foreground_color = (0, 0, 0);
    let background_color = (255, 255, 255);

    let mut chip8 = Chip8::build("", 1000, 10, background_color, foreground_color, true);
    chip8.run();
}
