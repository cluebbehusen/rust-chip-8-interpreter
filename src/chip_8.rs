use sdl2::{self, event::Event, keyboard::Keycode};
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
    sdl_context: sdl2::Sdl,
    debug: bool,
    instruction_time: u128,
    last_instruction_time: u128,
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

        let last_instruction_time = get_epoch_ns();
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

            sdl_context,
            display,
            debug,
            last_instruction_time,
            instruction_time,
            update_display: false,
        }
    }

    pub fn run(&mut self) {
        let mut event_pump = self.sdl_context.event_pump().unwrap();

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    Event::KeyDown {
                        keycode: Some(Keycode::Return),
                        ..
                    } => {
                        if self.debug {
                            self.cycle();
                        }
                    }
                    _ => {}
                }
            }

            let valid_cycle_time =
                get_epoch_ns() - self.last_instruction_time >= self.instruction_time;
            if valid_cycle_time && !self.debug {
                self.cycle();
                self.last_instruction_time = get_epoch_ns();
            }
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

    fn cycle(&mut self) {
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
            for i in 0..constants::REGISTER_COUNT {
                print!("V{:X}: {:X} | ", i, self.registers[i]);
            }
            println!("I: {:X}", self.index_register);
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
            0x20 => self.call_subroutine(parsed_instruction.nnn),
            0x30 => self.skip_if_equal(parsed_instruction.x, parsed_instruction.nn),
            0x40 => self.skip_if_not_equal(parsed_instruction.x, parsed_instruction.nn),
            0x50 => self.skip_if_registers_equal(parsed_instruction.x, parsed_instruction.y),
            0x60 => self.set_register(parsed_instruction.x, parsed_instruction.nn),
            0x70 => self.add_to_register(parsed_instruction.x, parsed_instruction.nn),
            0x80 => match parsed_instruction.n {
                0x00 => todo!("set"),
                0x01 => todo!("or"),
                0x02 => todo!("and"),
                0x03 => todo!("xor"),
                0x04 => todo!("add"),
                0x05 => todo!("sub"),
                0x06 => todo!("shift"),
                0x07 => todo!("sub"),
                0x0E => todo!("shift"),
                _ => panic!(
                    "Unrecognized fourth nibble: {:X} for opcode: {:X}",
                    parsed_instruction.n, parsed_instruction.opcode
                ),
            },
            0x90 => self.skip_if_registers_not_equal(parsed_instruction.x, parsed_instruction.y),
            0xA0 => self.set_index_register(parsed_instruction.nnn),
            0xD0 => self.display(
                parsed_instruction.x,
                parsed_instruction.y,
                parsed_instruction.n,
            ),
            _ => panic!("Unrecognized opcode: {:X}", parsed_instruction.opcode),
        }

        if self.update_display {
            self.display.render_buffer(self.display_buffer);
            self.update_display = false;
        }
    }

    // 0x00E0
    fn clear_screen(&mut self) {
        self.display_buffer = [false; constants::DISPLAY_LEN];
        self.update_display = true;
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

    // 0x2NNN
    fn call_subroutine(&mut self, address: u16) {
        self.stack_pointer += 1;
        self.stack[self.stack_pointer as usize] = self.program_counter as u16;
        self.program_counter = address as usize;
    }

    // 0x3XNN
    fn skip_if_equal(&mut self, register: u8, value: u8) {
        if self.registers[register as usize] == value {
            self.program_counter += 2;
        }
    }

    // 0x4XNN
    fn skip_if_not_equal(&mut self, register: u8, value: u8) {
        if self.registers[register as usize] != value {
            self.program_counter += 2;
        }
    }

    // 0x5XY0
    fn skip_if_registers_equal(&mut self, x_register: u8, y_register: u8) {
        if self.registers[x_register as usize] == self.registers[y_register as usize] {
            self.program_counter += 2;
        }
    }

    // 0x6XNN
    fn set_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = value;
    }

    // 0x7NN
    fn add_to_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = self.registers[register as usize].wrapping_add(value);
    }

    // 9XY0
    fn skip_if_registers_not_equal(&mut self, x_register: u8, y_register: u8) {
        if self.registers[x_register as usize] != self.registers[y_register as usize] {
            self.program_counter += 2;
        }
    }

    // 0xANNN
    fn set_index_register(&mut self, value: u16) {
        self.index_register = value;
    }

    // 0xDXYN
    fn display(&mut self, x_register: u8, y_register: u8, height: u8) {
        let x_coordinate = self.registers[x_register as usize] % constants::DISPLAY_WIDTH as u8;
        let y_coordinate = self.registers[y_register as usize] % constants::DISPLAY_HEIGHT as u8;
        self.registers[0x0F] = 0;

        for row in 0..height {
            let current_y_coordinate = (y_coordinate + row) as usize;
            if current_y_coordinate >= constants::DISPLAY_HEIGHT {
                break;
            }

            let sprite_data = self.ram[(self.index_register + row as u16) as usize];
            for column in 0..8 {
                let current_x_coordinate = (x_coordinate + column) as usize;
                if current_x_coordinate >= constants::DISPLAY_WIDTH {
                    break;
                }

                let current_coordinate =
                    current_x_coordinate + current_y_coordinate * constants::DISPLAY_WIDTH;
                if self.display_buffer[current_coordinate] {
                    self.registers[0x0F] = 1;
                }

                let sprite_pixel = (sprite_data >> (7 - column)) & 0x01;
                if sprite_pixel == 1 {
                    self.display_buffer[current_coordinate] ^= true;
                }
            }
        }

        self.update_display = true;
    }
}
