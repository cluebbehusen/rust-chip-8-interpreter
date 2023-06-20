mod chip_8;
mod constants;
mod display;

use chip_8::{Chip8, Platform, Quirks};

fn main() {
    let foreground_color = (255, 255, 255);
    let background_color = (0, 0, 0);

    let quirks = Quirks::new(Platform::Chip8);

    let mut chip8 = Chip8::build(
        "",
        140_000,
        10,
        background_color,
        foreground_color,
        false,
        quirks,
    );
    chip8.run();
}
