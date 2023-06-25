use clap::ValueEnum;
use rand;
use sdl2::{self, event::Event, keyboard::Keycode, keyboard::Scancode};
use std::collections::HashSet;
use std::time;

use crate::beep::Beep;
use crate::constants;
use crate::display::Display;

fn get_epoch_ns() -> u128 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn map_scancode_to_value(scancode: Scancode) -> Option<u8> {
    match scancode {
        Scancode::X => Some(0x00),
        Scancode::Num1 => Some(0x01),
        Scancode::Num2 => Some(0x02),
        Scancode::Num3 => Some(0x03),
        Scancode::Q => Some(0x04),
        Scancode::W => Some(0x05),
        Scancode::E => Some(0x06),
        Scancode::A => Some(0x07),
        Scancode::S => Some(0x08),
        Scancode::D => Some(0x09),
        Scancode::Z => Some(0x0A),
        Scancode::C => Some(0x0B),
        Scancode::Num4 => Some(0x0C),
        Scancode::R => Some(0x0D),
        Scancode::F => Some(0x0E),
        Scancode::V => Some(0x0F),
        _ => None,
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Platform {
    Chip8,
    SuperChip,
}

pub struct Quirks {
    reset_flag: bool,
    increment_index_register: bool,
    shift_in_place: bool,
    jump_plus_x_register: bool,
}

impl Quirks {
    pub fn new(platform: Platform) -> Self {
        match platform {
            Platform::Chip8 => Quirks {
                reset_flag: true,
                increment_index_register: true,
                shift_in_place: false,
                jump_plus_x_register: false,
            },
            Platform::SuperChip => Quirks {
                reset_flag: false,
                increment_index_register: false,
                shift_in_place: true,
                jump_plus_x_register: true,
            },
        }
    }
}

struct ParsedInstruction {
    opcode: u8,
    x: u8,
    y: u8,
    n: u8,
    nn: u8,
    nnn: u16,
}

impl ParsedInstruction {
    pub fn build(instruction: u16) -> ParsedInstruction {
        ParsedInstruction {
            opcode: ((instruction & 0xF000) >> 8) as u8,
            x: ((instruction & 0x0F00) >> 8) as u8,
            y: ((instruction & 0x00F0) >> 4) as u8,
            n: (instruction & 0x000F) as u8,
            nn: (instruction & 0x00FF) as u8,
            nnn: instruction & 0x0FFF,
        }
    }
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
    beep: Beep,
    sdl_context: sdl2::Sdl,
    debug: bool,
    instruction_time: u128,
    quirks: Quirks,

    last_instruction_time: u128,
    last_decrement_timer_time: u128,
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
        quirks: Quirks,
    ) -> Self {
        let bytes = std::fs::read(rom_file)
            .unwrap_or_else(|error| panic!("Failed to read file: {:?}", error));

        let mut ram = [0; constants::RAM_LEN];
        ram[constants::FONT_START..constants::FONT_END].copy_from_slice(&constants::FONT);
        let program_end = constants::PROGRAM_START + bytes.len();
        ram[constants::PROGRAM_START..program_end].copy_from_slice(&bytes);

        let current_epoch_ns = get_epoch_ns();
        let last_instruction_time = current_epoch_ns;
        let last_decrement_timer_time = current_epoch_ns;
        let sdl_context = sdl2::init().unwrap();
        let display = Display::build(&sdl_context, scale, background_color, foreground_color);
        let beep = Beep::build(&sdl_context);

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
            beep,
            display,
            debug,
            instruction_time,
            quirks,

            last_instruction_time,
            last_decrement_timer_time,
            update_display: false,
        }
    }

    pub fn run(&mut self) {
        let mut event_pump = self.sdl_context.event_pump().unwrap();

        'running: loop {
            let current_epoch_ns = get_epoch_ns();
            let valid_decrement_timer_time = current_epoch_ns - self.last_decrement_timer_time
                >= constants::TIMER_DECREMENT_TIME;
            if valid_decrement_timer_time {
                if self.delay_timer > 0 {
                    self.delay_timer -= 1;
                }
                if self.sound_timer > 0 {
                    self.beep.play();
                    self.sound_timer -= 1;
                } else {
                    self.beep.stop();
                }
                self.last_decrement_timer_time = current_epoch_ns;
            }

            let pressed_keys: HashSet<u8> = event_pump
                .keyboard_state()
                .pressed_scancodes()
                .filter_map(map_scancode_to_value)
                .collect();

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
                            self.cycle(&pressed_keys);
                        }
                    }
                    _ => {}
                }
            }

            let valid_cycle_time =
                current_epoch_ns - self.last_instruction_time >= self.instruction_time;
            if valid_cycle_time && !self.debug {
                self.cycle(&pressed_keys);
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

    fn cycle(&mut self, pressed_keys: &HashSet<u8>) {
        let instruction = self.fetch_instruction();
        let parsed_instruction = ParsedInstruction::build(instruction);

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
            0x10 => self.jump_to_address(parsed_instruction.nnn),
            0x20 => self.call_subroutine_at_address(parsed_instruction.nnn),
            0x30 => self.skip_if_equal_to_value(parsed_instruction.x, parsed_instruction.nn),
            0x40 => self.skip_if_not_equal_to_value(parsed_instruction.x, parsed_instruction.nn),
            0x50 => self.skip_if_equal_to_register(parsed_instruction.x, parsed_instruction.y),
            0x60 => self.set_register_to_value(parsed_instruction.x, parsed_instruction.nn),
            0x70 => self.add_value_to_register(parsed_instruction.x, parsed_instruction.nn),
            0x80 => match parsed_instruction.n {
                0x00 => self.set_register_to_register(parsed_instruction.x, parsed_instruction.y),
                0x01 => self.or_register_with_register(parsed_instruction.x, parsed_instruction.y),
                0x02 => self.and_register_with_register(parsed_instruction.x, parsed_instruction.y),
                0x03 => self.xor_register_with_register(parsed_instruction.x, parsed_instruction.y),
                0x04 => self.add_register_to_register(parsed_instruction.x, parsed_instruction.y),
                0x05 => {
                    self.subtract_register_from_register(parsed_instruction.x, parsed_instruction.y)
                }
                0x06 => self.set_register_to_right_shifted_register(
                    parsed_instruction.x,
                    parsed_instruction.y,
                ),
                0x07 => self.subtract_register_from_register_flipped(
                    parsed_instruction.x,
                    parsed_instruction.y,
                ),
                0x0E => self.set_register_to_left_shifted_register(
                    parsed_instruction.x,
                    parsed_instruction.y,
                ),
                _ => panic!(
                    "Unrecognized fourth nibble: {:X} for opcode: {:X}",
                    parsed_instruction.n, parsed_instruction.opcode
                ),
            },
            0x90 => self.skip_if_not_equal_to_register(parsed_instruction.x, parsed_instruction.y),
            0xA0 => self.set_index_register_to_value(parsed_instruction.nnn),
            0xB0 => self.jump_to_address_with_offset(parsed_instruction.x, parsed_instruction.nnn),
            0xC0 => self.set_register_to_random(parsed_instruction.x, parsed_instruction.nn),
            0xD0 => self.display(
                parsed_instruction.x,
                parsed_instruction.y,
                parsed_instruction.n,
            ),
            0xE0 => match parsed_instruction.nn {
                0x9E => self.skip_if_key_pressed(parsed_instruction.x, pressed_keys),
                0xA1 => self.skip_if_key_not_pressed(parsed_instruction.x, pressed_keys),
                _ => panic!(
                    "Unrecognized second byte: {:X} for opcode: {:X}",
                    parsed_instruction.nn, parsed_instruction.opcode
                ),
            },
            0xF0 => match parsed_instruction.nn {
                0x07 => self.set_register_to_delay_timer(parsed_instruction.x),
                0x0A => self.set_register_to_key_with_wait(parsed_instruction.x, pressed_keys),
                0x15 => self.set_delay_timer_to_register(parsed_instruction.x),
                0x18 => self.set_sound_timer_to_register(parsed_instruction.x),
                0x1E => self.add_register_to_index_register(parsed_instruction.x),
                0x29 => self.set_index_register_to_font_sprite(parsed_instruction.x),
                0x33 => self.set_index_register_to_bcd(parsed_instruction.x),
                0x55 => self.store_registers_in_memory(parsed_instruction.x),
                0x65 => self.load_registers_from_memory(parsed_instruction.x),
                _ => panic!(
                    "Unrecognized second byte: {:X} for opcode: {:X}",
                    parsed_instruction.nn, parsed_instruction.opcode
                ),
            },
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
    fn jump_to_address(&mut self, address: u16) {
        self.program_counter = address as usize;
    }

    // 0x2NNN
    fn call_subroutine_at_address(&mut self, address: u16) {
        self.stack_pointer += 1;
        self.stack[self.stack_pointer as usize] = self.program_counter as u16;
        self.program_counter = address as usize;
    }

    // 0x3XNN
    fn skip_if_equal_to_value(&mut self, register: u8, value: u8) {
        if self.registers[register as usize] == value {
            self.program_counter += 2;
        }
    }

    // 0x4XNN
    fn skip_if_not_equal_to_value(&mut self, register: u8, value: u8) {
        if self.registers[register as usize] != value {
            self.program_counter += 2;
        }
    }

    // 0x5XY0
    fn skip_if_equal_to_register(&mut self, x_register: u8, y_register: u8) {
        if self.registers[x_register as usize] == self.registers[y_register as usize] {
            self.program_counter += 2;
        }
    }

    // 0x6XNN
    fn set_register_to_value(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = value;
    }

    // 0x7XNN
    fn add_value_to_register(&mut self, register: u8, value: u8) {
        self.registers[register as usize] = self.registers[register as usize].wrapping_add(value);
    }

    // 0x8XY0
    fn set_register_to_register(&mut self, x_register: u8, y_register: u8) {
        self.registers[x_register as usize] = self.registers[y_register as usize];
    }

    // 0x8XY1
    fn or_register_with_register(&mut self, x_register: u8, y_register: u8) {
        self.registers[x_register as usize] |= self.registers[y_register as usize];
        if self.quirks.reset_flag {
            self.registers[0x0F] = 0;
        }
    }

    // 0x8XY2
    fn and_register_with_register(&mut self, x_register: u8, y_register: u8) {
        self.registers[x_register as usize] &= self.registers[y_register as usize];
        if self.quirks.reset_flag {
            self.registers[0x0F] = 0;
        }
    }

    // 0x8XY3
    fn xor_register_with_register(&mut self, x_register: u8, y_register: u8) {
        self.registers[x_register as usize] ^= self.registers[y_register as usize];
        if self.quirks.reset_flag {
            self.registers[0x0F] = 0;
        }
    }

    // 0x8XY4
    fn add_register_to_register(&mut self, x_register: u8, y_register: u8) {
        let (result, overflow) = self.registers[x_register as usize]
            .overflowing_add(self.registers[y_register as usize]);
        self.registers[x_register as usize] = result;
        self.registers[0x0F] = overflow as u8;
    }

    // 0x8XY5
    fn subtract_register_from_register(&mut self, x_register: u8, y_register: u8) {
        let (result, overflow) = self.registers[x_register as usize]
            .overflowing_sub(self.registers[y_register as usize]);
        self.registers[x_register as usize] = result;
        self.registers[0x0F] = !overflow as u8;
    }

    // 0x8XY6
    fn set_register_to_right_shifted_register(&mut self, x_register: u8, y_register: u8) {
        if !self.quirks.shift_in_place {
            self.registers[x_register as usize] = self.registers[y_register as usize];
        }
        let shift = self.registers[x_register as usize] & 0x01;
        self.registers[x_register as usize] >>= 1;
        self.registers[0x0F] = shift;
    }

    // 0x8XY7
    fn subtract_register_from_register_flipped(&mut self, x_register: u8, y_register: u8) {
        let (result, overflow) = self.registers[y_register as usize]
            .overflowing_sub(self.registers[x_register as usize]);
        self.registers[x_register as usize] = result;
        self.registers[0x0F] = !overflow as u8;
    }

    // 0x8XYE
    fn set_register_to_left_shifted_register(&mut self, x_register: u8, y_register: u8) {
        if !self.quirks.shift_in_place {
            self.registers[x_register as usize] = self.registers[y_register as usize];
        }
        let shift = (self.registers[x_register as usize] & 0x80) >> 7;
        self.registers[x_register as usize] <<= 1;
        self.registers[0x0F] = shift;
    }

    // 9XY0
    fn skip_if_not_equal_to_register(&mut self, x_register: u8, y_register: u8) {
        if self.registers[x_register as usize] != self.registers[y_register as usize] {
            self.program_counter += 2;
        }
    }

    // 0xANNN
    fn set_index_register_to_value(&mut self, value: u16) {
        self.index_register = value;
    }

    // 0xBNNN
    fn jump_to_address_with_offset(&mut self, x_register: u8, address: u16) {
        let offset = match self.quirks.jump_plus_x_register {
            true => self.registers[x_register as usize],
            false => self.registers[0],
        } as u16;
        self.program_counter = (address + offset) as usize;
    }

    // 0xCXNN
    fn set_register_to_random(&mut self, register: u8, value: u8) {
        let random_value = rand::random::<u8>();
        self.registers[register as usize] = random_value & value;
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

    // 0xEX9E
    fn skip_if_key_pressed(&mut self, register: u8, pressed_keys: &HashSet<u8>) {
        let key = self.registers[register as usize];
        if pressed_keys.contains(&key) {
            self.program_counter += 2;
        }
    }

    // 0xEXA1
    fn skip_if_key_not_pressed(&mut self, register: u8, pressed_keys: &HashSet<u8>) {
        let key = self.registers[register as usize];
        if !pressed_keys.contains(&key) {
            self.program_counter += 2;
        }
    }

    // 0xFX07
    fn set_register_to_delay_timer(&mut self, register: u8) {
        self.registers[register as usize] = self.delay_timer;
    }

    // 0xFX0A
    fn set_register_to_key_with_wait(&mut self, register: u8, pressed_keys: &HashSet<u8>) {
        if pressed_keys.is_empty() {
            self.program_counter -= 2;
        } else {
            let key = pressed_keys.iter().next().unwrap();
            self.registers[register as usize] = *key;
        }
    }

    // 0xFX15
    fn set_delay_timer_to_register(&mut self, register: u8) {
        self.delay_timer = self.registers[register as usize];
    }

    // 0xFX18
    fn set_sound_timer_to_register(&mut self, register: u8) {
        self.sound_timer = self.registers[register as usize];
    }

    // 0xFX1E
    fn add_register_to_index_register(&mut self, register: u8) {
        self.index_register += self.registers[register as usize] as u16;
    }

    // 0xFX29
    fn set_index_register_to_font_sprite(&mut self, register: u8) {
        let font_sprite = self.registers[register as usize] * 5;
        self.index_register = font_sprite as u16 + constants::FONT_START as u16;
    }

    // 0xFX33
    fn set_index_register_to_bcd(&mut self, register: u8) {
        let value = self.registers[register as usize];
        let hundreds = value / 100;
        let tens = (value / 10) % 10;
        let ones = value % 10;

        self.ram[self.index_register as usize] = hundreds;
        self.ram[self.index_register as usize + 1] = tens;
        self.ram[self.index_register as usize + 2] = ones;
    }

    // 0xFX55
    fn store_registers_in_memory(&mut self, x: u8) {
        for i in 0..=x {
            if self.quirks.increment_index_register {
                self.ram[self.index_register as usize] = self.registers[i as usize];
                self.index_register += 1;
            } else {
                self.ram[self.index_register as usize + i as usize] = self.registers[i as usize];
            }
        }
    }

    // 0xFX65
    fn load_registers_from_memory(&mut self, x: u8) {
        for i in 0..=x {
            if self.quirks.increment_index_register {
                self.registers[i as usize] = self.ram[self.index_register as usize];
                self.index_register += 1;
            } else {
                self.registers[i as usize] = self.ram[self.index_register as usize + i as usize];
            }
        }
    }
}
