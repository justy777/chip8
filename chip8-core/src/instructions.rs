use crate::{Chip8, FONT_SET_START_ADDRESS, KEY_COUNT, VIDEO_HEIGHT, VIDEO_WIDTH};
use rand::Rng;

impl Chip8 {
    // 00E0: CLS
    pub(crate) fn op_00e0(&mut self) {
        self.video.fill(0);
    }

    //00EE: RET
    pub(crate) const fn op_00ee(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    // 1nnn: JP addr
    pub(crate) const fn op_1nnn(&mut self) {
        let address = self.opcode & 0xFFF;
        self.pc = address;
    }

    // 2nnn: CALL addr
    pub(crate) const fn op_2nnn(&mut self) {
        let address = self.opcode & 0xFFF;
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = address;
    }

    // 3xkk: SE Vx, byte
    pub(crate) const fn op_3xkk(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let byte = (self.opcode & 0xFF) as u8;

        if self.registers[vx as usize] == byte {
            self.pc += 2;
        }
    }

    // 4xkk: SNE Vx, byte
    pub(crate) const fn op_4xkk(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let byte = (self.opcode & 0xFF) as u8;

        if self.registers[vx as usize] != byte {
            self.pc += 2;
        }
    }

    // 5xy0: SE Vx, Vy
    pub(crate) const fn op_5xy0(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        if self.registers[vx as usize] == self.registers[vy as usize] {
            self.pc += 2;
        }
    }

    // 6xkk: LD Vx, byte
    pub(crate) const fn op_6xkk(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let byte = (self.opcode & 0xFF) as u8;

        self.registers[vx as usize] = byte;
    }

    // 7xkk: ADD Vx, byte
    pub(crate) const fn op_7xkk(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let byte = (self.opcode & 0xFF) as u8;

        self.registers[vx as usize] = self.registers[vx as usize].wrapping_add(byte);
    }

    // 8xy0: LD Vx, Vy
    pub(crate) const fn op_8xy0(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        self.registers[vx as usize] = self.registers[vy as usize];
    }

    // 8xy1: OR Vx, Vy
    pub(crate) const fn op_8xy1(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        if self.quirks.vf_reset {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] |= self.registers[vy as usize];
    }

    // 8xy2: AND Vx, Vy
    pub(crate) const fn op_8xy2(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        if self.quirks.vf_reset {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] &= self.registers[vy as usize];
    }

    // 8xy3: XOR Vx, Vy
    pub(crate) const fn op_8xy3(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        if self.quirks.vf_reset {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] ^= self.registers[vy as usize];
    }

    // 8xy4: ADD Vx, Vy
    pub(crate) const fn op_8xy4(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        let (sum, did_overflow) =
            self.registers[vx as usize].overflowing_add(self.registers[vy as usize]);

        self.registers[vx as usize] = sum;

        if did_overflow {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }
    }

    // 8xy5: SUB Vx, Vy
    pub(crate) const fn op_8xy5(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        let (difference, did_overflow) =
            self.registers[vx as usize].overflowing_sub(self.registers[vy as usize]);

        self.registers[vx as usize] = difference;

        if did_overflow {
            self.registers[0xF] = 0;
        } else {
            self.registers[0xF] = 1;
        }
    }

    // 8xy6: SHR Vx
    pub(crate) const fn op_8xy6(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        if !self.quirks.shifting {
            self.registers[vx as usize] = self.registers[vy as usize];
        }

        let new_flag = self.registers[vx as usize] & 0x1;
        self.registers[vx as usize] >>= 1;

        self.registers[0xF] = new_flag;
    }

    // 8xy7: SUBN Vx, Vy
    pub(crate) const fn op_8xy7(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        let (difference, did_overflow) =
            self.registers[vy as usize].overflowing_sub(self.registers[vx as usize]);

        self.registers[vx as usize] = difference;

        if did_overflow {
            self.registers[0xF] = 0;
        } else {
            self.registers[0xF] = 1;
        }
    }

    // 8xyE: SHL Vx {, Vy}
    pub(crate) const fn op_8xye(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        if !self.quirks.shifting {
            self.registers[vx as usize] = self.registers[vy as usize];
        }

        let new_flag = (self.registers[vx as usize] & 0x80) >> 7;
        self.registers[vx as usize] <<= 1;

        self.registers[0xF] = new_flag;
    }

    // 9xy0: SNE Vx, Vy
    pub(crate) const fn op_9xy0(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;

        if self.registers[vx as usize] != self.registers[vy as usize] {
            self.pc += 2;
        }
    }

    // Annn: LD I, addr
    pub(crate) const fn op_annn(&mut self) {
        let address = self.opcode & 0xFFF;
        self.index = address;
    }

    // Bnnn: JP V0, addr
    pub(crate) const fn op_bnnn(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let address = self.opcode & 0xFFF;
        if self.quirks.jumping {
            self.pc = address + self.registers[vx as usize] as u16;
        } else {
            self.pc = self.registers[0] as u16 + address;
        }
    }

    // Cxkk: RND Vx, byte
    pub(crate) fn op_cxkk(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let byte = (self.opcode & 0xFF) as u8;

        let mut rng = rand::rng();
        let rand_byte: u8 = rng.random();

        self.registers[vx as usize] = rand_byte & byte;
    }

    // Dxyn: DRW Vx, Vy, nibble
    pub(crate) fn op_dxyn(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let vy = ((self.opcode & 0xF0) >> 4) as u8;
        let height = (self.opcode & 0xF) as u8;

        let x_pos = self.registers[vx as usize] % VIDEO_WIDTH as u8;
        let y_pos = self.registers[vy as usize] % VIDEO_HEIGHT as u8;

        self.registers[0xF] = 0;

        for row in 0..height {
            let sprite_byte = self.memory[(self.index + row as u16) as usize];

            if self.quirks.clipping && (y_pos + row) as usize >= VIDEO_HEIGHT {
                break;
            }

            for col in 0..8 {
                let sprite_pixel = sprite_byte & (0x80 >> col);

                if self.quirks.clipping && (x_pos + col) as usize >= VIDEO_WIDTH {
                    break;
                }

                let wrapped_x_pos = (x_pos + col) as usize % VIDEO_WIDTH;
                let wrapped_y_pos = (y_pos + row) as usize % VIDEO_HEIGHT;
                let screen_index = wrapped_y_pos * VIDEO_WIDTH + wrapped_x_pos;

                let screen_pixel = &mut self.video[screen_index];

                if sprite_pixel != 0 {
                    if *screen_pixel == 0xFFFF_FFFF {
                        self.registers[0xF] = 1;
                    }

                    *screen_pixel ^= 0xFFFF_FFFF;
                }
            }
        }
    }

    // Ex9E: SKP vx
    pub(crate) const fn op_ex9e(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        let key = self.registers[vx as usize];

        if self.keypad[key as usize] != 0 {
            self.pc += 2;
        }
    }

    // ExA1: SKNP Vx
    pub(crate) const fn op_exa1(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        let key = self.registers[vx as usize];

        if self.keypad[key as usize] == 0 {
            self.pc += 2;
        }
    }

    // Fx07: LD Vx, DT
    pub(crate) const fn op_fx07(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        self.registers[vx as usize] = self.delay_timer;
    }

    // Fx0A: LD Vx, K
    pub(crate) fn op_fx0a(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        let mut done = false;

        if !self.quirks.release || self.pressed_key.is_none() {
            for i in 0..KEY_COUNT {
                if self.keypad[i] != 0 {
                    self.registers[vx as usize] = i as u8;
                    if !self.quirks.release {
                        done = true;
                    }
                    self.pressed_key = Some(i as u8);
                    break;
                }
            }
        }

        if self.quirks.release
            && self
                .pressed_key
                .is_some_and(|val| self.keypad[val as usize] == 0)
        {
            self.pressed_key = None;
            done = true;
        }

        if !done {
            self.pc -= 2;
        }
    }

    // Fx15: LD DT, Vx
    pub(crate) const fn op_fx15(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        self.delay_timer = self.registers[vx as usize];
    }

    // Fx18: LD ST, Vx
    pub(crate) const fn op_fx18(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        self.sound_timer = self.registers[vx as usize];
    }

    // Fx1E: ADD I, Vx
    pub(crate) const fn op_fx1e(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        self.index += self.registers[vx as usize] as u16;
    }

    // Fx29: LD F, Vx
    pub(crate) const fn op_fx29(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let digit = self.registers[vx as usize];

        self.index = (FONT_SET_START_ADDRESS as u16) + (5 * digit) as u16;
    }

    // Fx33: LD B, Vx
    pub(crate) const fn op_fx33(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;
        let mut value = self.registers[vx as usize];

        // Ones-place
        self.memory[(self.index + 2) as usize] = value % 10;
        value /= 10;

        // Tens-place
        self.memory[(self.index + 1) as usize] = value % 10;
        value /= 10;

        // Hundreds-place
        self.memory[self.index as usize] = value % 10;
    }

    // Fx55: LD [I], Vx
    pub(crate) fn op_fx55(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        for i in 0..=vx {
            self.memory[(self.index + i as u16) as usize] = self.registers[i as usize];
        }

        if self.quirks.memory {
            self.index = self.index + vx as u16 + 1;
        }
    }

    // Fx65: LD Vx, [I]
    pub(crate) fn op_fx65(&mut self) {
        let vx = ((self.opcode & 0xF00) >> 8) as u8;

        for i in 0..=vx {
            self.registers[i as usize] = self.memory[(self.index + i as u16) as usize];
        }

        if self.quirks.memory {
            self.index = self.index + vx as u16 + 1;
        }
    }
}
