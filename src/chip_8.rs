use sdl2;
use std::time;

use crate::constants;
use crate::display::Display;

fn get_epoch_ns() -> u128 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

struct ParsedInstruction {
    opcode: u8,
    x: u8,
    y: u8,
    n: u8,
    nn: u8,
    nnn: u16,
}

pub struct Chip8 {
    ram: [u8; constants::RAM_LEN],
    registers: [u8; constants::REGISTER_COUNT],
    stack: [u16; constants::STACK_LEN],
    delay_timer: u8,
    sound_timer: u8,
    index_register: u16,
    program_counter: usize,
    stack_pointer: u8,
    display_buffer: [bool; constants::DISPLAY_LEN],

    display: Display,
    debug: bool,
    instruction_time: u128,
    start_time: u128,
    update_display: bool,
}

impl Chip8 {
    pub fn build(
        rom_file: &str,
        instruction_time: u128,
        scale: u32,
        background_color: (u8, u8, u8),
        foreground_color: (u8, u8, u8),
        debug: bool,
    ) -> Self {
        let bytes = std::fs::read(rom_file)
            .unwrap_or_else(|error| panic!("Failed to read file: {:?}", error));

        let mut ram = [0; constants::RAM_LEN];
        ram[constants::FONT_START..constants::FONT_END].copy_from_slice(&constants::FONT);
        let program_end = constants::PROGRAM_START + bytes.len();
        ram[constants::PROGRAM_START..program_end].copy_from_slice(&bytes);

        let start_time = get_epoch_ns();
        let sdl_context = sdl2::init().unwrap();
        let display = Display::build(&sdl_context, scale, background_color, foreground_color);

        Chip8 {
            ram,
            registers: [0; constants::REGISTER_COUNT],
            stack: [0; constants::STACK_LEN],
            delay_timer: 0,
            sound_timer: 0,
            index_register: 0,
            program_counter: constants::PROGRAM_START,
            stack_pointer: 0,
            display_buffer: [false; constants::DISPLAY_LEN],
            display,

            debug,
            start_time,
            instruction_time,
            update_display: false,
        }
    }

    fn fetch_instruction(&mut self) -> u16 {
        let instruction_first_byte = self.ram[self.program_counter];
        let instruction_second_byte = self.ram[self.program_counter + 1];
        self.program_counter += 2;

        ((instruction_first_byte as u16) << 8) | instruction_second_byte as u16
    }

    fn parse_instruction(instruction: u16) -> ParsedInstruction {
        ParsedInstruction {
            opcode: ((instruction & 0xF000) >> 8) as u8,
            x: ((instruction & 0x0F00) >> 8) as u8,
            y: ((instruction & 0x00F0) >> 4) as u8,
            n: (instruction & 0x000F) as u8,
            nn: (instruction & 0x00FF) as u8,
            nnn: instruction & 0x0FFF,
        }
    }

    pub fn cycle(&mut self) {
        let instruction = self.fetch_instruction();
        let parsed_instruction = Chip8::parse_instruction(instruction);

        if self.debug {
            println!(
                "Instruction: {:04X} | Opcode: {:X} | X: {:X} | Y: {:X} | N: {:X} | NN: {:X} | NNN: {:X}",
                instruction,
                parsed_instruction.opcode,
                parsed_instruction.x,
                parsed_instruction.y,
                parsed_instruction.n,
                parsed_instruction.nn,
                parsed_instruction.nnn,
            );
        }

        match parsed_instruction.opcode {
            0x00 => match parsed_instruction.nn {
                0xE0 => self.clear_screen(),
                0xEE => self.return_from_subroutine(),
                _ => panic!(
                    "Unrecognized second byte: {:X} for opcode: {:X}",
                    parsed_instruction.nn, parsed_instruction.opcode
                ),
            },
            0x10 => self.jump(parsed_instruction.nnn),
            0x60 => self.set_register(parsed_instruction.x, parsed_instruction.nn),
            0x70 => self.add_to_register(parsed_instruction.x, parsed_instruction.nn),
            0xA0 => self.set_index_register(parsed_instruction.nnn),
            0xD0 => self.display(
                parsed_instruction.x,
                parsed_instruction.y,
                parsed_instruction.n,
            ),
            _ => panic!("Unrecognized opcode: {:X}", parsed_instruction.opcode),
        }
    }

    // 0x00E0
    fn clear_screen(&mut self) {
        self.display_buffer = [false; constants::DISPLAY_LEN];
        self.update_display = false;
    }

    // 0x00EE
    fn return_from_subroutine(&mut self) {
        if self.stack_pointer == 0 {
            panic!("Stack pointer is 0, cannot return from subroutine");
        }
        self.program_counter = self.stack[self.stack_pointer as usize] as usize;
        self.stack_pointer -= 1;
    }

    // 0x1NNN
    fn jump(&mut self, address: u16) {
        self.program_counter = address as usize;
    }

    // 0x6XNN
    fn set_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = value;
    }

    // 0x7NN
    fn add_to_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = self.registers[register as usize].wrapping_add(value);
    }

    // 0xANNN
    fn set_index_register(&mut self, value: u16) {
        self.index_register = value;
    }

    // 0xDXYN
    fn display(&mut self, x_register: u8, y_register: u8, height: u8) {
        todo!("Implement display")
    }
}
