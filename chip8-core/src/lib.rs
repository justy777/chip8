#![allow(clippy::cast_lossless)]

mod instructions;

const MEMORY_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
const STACK_LEVELS: usize = 16;
const KEY_COUNT: usize = 16;

pub const VIDEO_WIDTH: usize = 64;
pub const VIDEO_HEIGHT: usize = 32;

const FONT_SET_SIZE: usize = 80;
const FONT_SET_START_ADDRESS: usize = 0x50;
const START_ADDRESS: usize = 0x200;

const FONT_SET: [u8; FONT_SET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

#[derive(Debug)]
struct Quirks {
    /// The AND, OR and XOR opcodes (`8xy1`, `8xy2` and `8xy3`) reset the flags register to zero.
    vf_reset: bool,
    /// The save and load opcodes (`Fx55` and `Fx65`) increment the index register.
    memory: bool,
    /// Sprites drawn at the bottom edge of the screen get clipped instead of wrapping around the screen.
    clipping: bool,
    /// The shift opcodes (`8xy6` and `8xyE`) only operate on vX instead of storing the shifted version of vY in vX.
    shifting: bool,
    /// The jump instruction (`Bnnn`) doesn't use v0, but vX instead where X is the highest nibble of nnn.
    jumping: bool,
    /// The get key instruction (`Fx0A`) waits for a key press and key up.
    release: bool,
}

impl Quirks {
    pub const fn new() -> Self {
        Self {
            vf_reset: true,
            memory: true,
            clipping: true,
            shifting: false,
            jumping: false,
            release: true,
        }
    }
}

#[derive(Debug)]
pub enum ExecuteError {
    UndefinedInstruction(u16),
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UndefinedInstruction(opcode) => write!(f, "Undefined instruction {opcode}"),
        }
    }
}

impl std::error::Error for ExecuteError {}

#[derive(Debug)]
pub struct Chip8 {
    memory: [u8; MEMORY_SIZE],
    registers: [u8; REGISTER_COUNT],
    index: u16,
    pc: u16,
    sp: u8,
    stack: [u16; STACK_LEVELS],
    delay_timer: u8,
    sound_timer: u8,
    opcode: u16,
    quirks: Quirks,
    // Used to check if pressed key is released
    pressed_key: Option<u8>,
    pub keypad: [u8; KEY_COUNT],
    pub video: [u32; VIDEO_WIDTH * VIDEO_HEIGHT],
}

impl Chip8 {
    #[must_use]
    pub fn new() -> Self {
        let mut memory = [0; MEMORY_SIZE];

        memory[FONT_SET_START_ADDRESS..(FONT_SET_START_ADDRESS + FONT_SET_SIZE)]
            .copy_from_slice(&FONT_SET[..]);

        Self {
            memory,
            registers: [0; REGISTER_COUNT],
            index: 0,
            pc: START_ADDRESS as u16,
            sp: 0,
            stack: [0; STACK_LEVELS],
            delay_timer: 0,
            sound_timer: 0,
            opcode: 0,
            quirks: Quirks::new(),
            pressed_key: None,
            keypad: [0; KEY_COUNT],
            video: [0; VIDEO_WIDTH * VIDEO_HEIGHT],
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.memory[START_ADDRESS..(START_ADDRESS + rom.len())].copy_from_slice(rom);
    }

    pub fn emulate(&mut self) -> Result<(), ExecuteError> {
        // Fetch
        self.opcode = ((self.memory[self.pc as usize] as u16) << 8)
            | (self.memory[(self.pc + 1) as usize] as u16);

        // Increment the PC before we execute anything
        self.pc += 2;

        // Decode and Execute
        self.execute()?;

        // Decrement the delay timer if it's been set
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        // Decrement the sound timer if it's been set
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }

        Ok(())
    }

    fn execute(&mut self) -> Result<(), ExecuteError> {
        match self.opcode {
            0x00E0 => self.op_00e0(),
            0x00EE => self.op_00ee(),
            n if n & 0xF000 == 0x1000 => self.op_1nnn(),
            n if n & 0xF000 == 0x2000 => self.op_2nnn(),
            n if n & 0xF000 == 0x3000 => self.op_3xkk(),
            n if n & 0xF000 == 0x4000 => self.op_4xkk(),
            n if n & 0xF00F == 0x5000 => self.op_5xy0(),
            n if n & 0xF000 == 0x6000 => self.op_6xkk(),
            n if n & 0xF000 == 0x7000 => self.op_7xkk(),
            n if n & 0xF00F == 0x8000 => self.op_8xy0(),
            n if n & 0xF00F == 0x8001 => self.op_8xy1(),
            n if n & 0xF00F == 0x8002 => self.op_8xy2(),
            n if n & 0xF00F == 0x8003 => self.op_8xy3(),
            n if n & 0xF00F == 0x8004 => self.op_8xy4(),
            n if n & 0xF00F == 0x8005 => self.op_8xy5(),
            n if n & 0xF00F == 0x8006 => self.op_8xy6(),
            n if n & 0xF00F == 0x8007 => self.op_8xy7(),
            n if n & 0xF00F == 0x800E => self.op_8xye(),
            n if n & 0xF00F == 0x9000 => self.op_9xy0(),
            n if n & 0xF000 == 0xA000 => self.op_annn(),
            n if n & 0xF000 == 0xB000 => self.op_bnnn(),
            n if n & 0xF000 == 0xC000 => self.op_cxkk(),
            n if n & 0xF000 == 0xD000 => self.op_dxyn(),
            n if n & 0xF0FF == 0xE09E => self.op_ex9e(),
            n if n & 0xF0FF == 0xE0A1 => self.op_exa1(),
            n if n & 0xF0FF == 0xF007 => self.op_fx07(),
            n if n & 0xF0FF == 0xF00A => self.op_fx0a(),
            n if n & 0xF0FF == 0xF015 => self.op_fx15(),
            n if n & 0xF0FF == 0xF018 => self.op_fx18(),
            n if n & 0xF0FF == 0xF01E => self.op_fx1e(),
            n if n & 0xF0FF == 0xF029 => self.op_fx29(),
            n if n & 0xF0FF == 0xF033 => self.op_fx33(),
            n if n & 0xF0FF == 0xF055 => self.op_fx55(),
            n if n & 0xF0FF == 0xF065 => self.op_fx65(),
            _ => return Err(ExecuteError::UndefinedInstruction(self.opcode)),
        }
        Ok(())
    }
}

impl Default for Chip8 {
    fn default() -> Self {
        Self::new()
    }
}
