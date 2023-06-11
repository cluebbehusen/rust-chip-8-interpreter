use crate::constants;

pub struct Chip8 {
    ram: [u8; constants::RAM_LEN],
    registers: [u8; constants::REGISTER_COUNT],
    stack: [u16; constants::STACK_LEN],
    delay_timer: u8,
    sound_timer: u8,
    index_register: u16,
    program_counter: usize,
    stack_pointer: u8,
    display: [bool; constants::DISPLAY_LEN],

    debug: bool,
    instruction_time: u128,
}

impl Chip8 {
    pub fn build(rom_file: &str, instruction_time: u128, debug: bool) -> Self {
        let bytes = std::fs::read(rom_file)
            .unwrap_or_else(|error| panic!("Failed to read file: {:?}", error));

        let mut ram = [0; constants::RAM_LEN];
        ram[constants::FONT_START..constants::FONT_END].copy_from_slice(&constants::FONT);
        let program_end = constants::PROGRAM_START + bytes.len();
        ram[constants::PROGRAM_START..program_end].copy_from_slice(&bytes);

        Chip8 {
            ram,
            registers: [0; constants::REGISTER_COUNT],
            stack: [0; constants::STACK_LEN],
            delay_timer: 0,
            sound_timer: 0,
            index_register: 0,
            program_counter: constants::PROGRAM_START,
            stack_pointer: 0,
            display: [false; constants::DISPLAY_LEN],

            debug,
            instruction_time,
        }
    }
}
