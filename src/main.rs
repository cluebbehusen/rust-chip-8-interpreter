mod beep;
mod chip_8;
mod constants;
mod display;

use clap::Parser;

use chip_8::{Chip8, Platform, Quirks};

/// A CHIP-8 interpreter written in Rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the ROM file to load
    rom_file: String,

    /// Platform to emulate
    #[clap(value_enum, short, long, default_value_t = Platform::Chip8)]
    platform: Platform,

    /// The instruction time in nanoseconds
    #[arg(short, long, default_value_t = 140_000)]
    instruction_time: u128,

    /// The display scale
    #[arg(short, long, default_value_t = 10)]
    scale: u32,

    /// Debug mode (displays registers and waits each cycle)
    #[arg(short, long, default_value_t = false)]
    debug: bool,
}

fn main() {
    let args = Args::parse();

    let foreground_color = (255, 255, 255);
    let background_color = (0, 0, 0);

    let quirks = Quirks::new(args.platform);

    let mut chip8 = Chip8::build(
        &args.rom_file,
        args.instruction_time,
        args.scale,
        background_color,
        foreground_color,
        args.debug,
        quirks,
    );

    chip8.run();
}
