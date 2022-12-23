use std::fs::File;
use std::io::{self, Read};

use log::debug;

use crate::display::Display;
use crate::lsfr::Lsfr;

#[derive(Debug)]
pub struct Chip8 {
    registers: [u8; 16],
    memory: [u8; Self::MEMORY_SIZE],
    index: usize,
    pc: usize,
    stack: [u16; 16],
    sp: usize,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [u8; 16],
    display: Display,
    lsfr: Lsfr,
}

enum PC {
    Next,
    Skip,
    Jump(usize),
}

impl Chip8 {
    const MEMORY_SIZE: usize = 4096;
    const START_ADDRESS: usize = 0x200;
    const FONTSET_START_ADDRESS: usize = 0x50;
    const FONTSET: [u8; 80] = [
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

    const fn start_memory() -> [u8; Self::MEMORY_SIZE] {
        let mut memory = [0; Self::MEMORY_SIZE];

        let mut i = 0;
        while i < Self::FONTSET.len() {
            memory[i + Self::FONTSET_START_ADDRESS] = Self::FONTSET[i];
            i += 1;
        }

        memory
    }

    pub fn read_rom(filename: &str) -> io::Result<Self> {
        let mut f = File::open(filename)?;
        let mut memory = Self::start_memory();

        let n = f.read(&mut memory[Self::START_ADDRESS..])?;
        debug!("Read {} bytes", n);

        Ok(Self {
            registers: [0; 16],
            memory,
            index: 0,
            pc: Self::START_ADDRESS,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [0; 16],
            display: Display::new(),
            lsfr: Lsfr::new(),
        })
    }

    fn gen_random(&mut self) -> u8 {
        self.lsfr.gen()
    }

    fn process_instruction(&mut self, instruction: u16) {
        let x = instruction.to_be_bytes();
        let o1: u8 = x[0] >> 4;
        let o2: u8 = x[0] & 0xf;
        let o3: u8 = x[1] >> 4;
        let o4: u8 = x[1] & 0xf;

        #[inline(always)]
        fn nnn(n1: u8, n2: u8, n3: u8) -> u16 {
            ((n1 as u16) << 8) + ((n2 as u16) << 4) + n3 as u16
        }
        #[inline(always)]
        fn var(x1: u8, x2: u8) -> u8 {
            ((x1 as u8) << 4) + x2 as u8
        }

        debug!("instruction: {:x}{:x}{:x}{:x}", o1, o2, o3, o4);

        let pc_change = match (o1, o2, o3, o4) {
            // 00E0 - CLS
            (0x0, 0x0, 0xE, 0x0) => {
                debug!("00E0 - CLS");

                self.display.clear();
                PC::Next
            }
            // 00EE - RET
            (0x0, 0x0, 0xE, 0xE) => {
                debug!("00EE - RET");

                let pc = self.stack[self.sp as usize - 1] as usize;
                self.sp -= 1;
                PC::Jump(pc + 2)
            }
            // 1nnn - JP addr
            (0x1, n1, n2, n3) => {
                let nnn = nnn(n1, n2, n3) as usize;
                debug!("1nnn - JP {:x}", nnn);

                PC::Jump(nnn)
            }
            // 2nnn - CALL addr
            (0x2, n1, n2, n3) => {
                let nnn = nnn(n1, n2, n3) as usize;
                debug!("2nnn - CALL {:x}", nnn);

                self.stack[self.sp] = self.pc as u16;
                self.sp += 1;
                PC::Jump(nnn)
            }
            // 3xkk - SE Vx, byte
            (0x3, x, k1, k2) => {
                let vx = self.registers[x as usize];
                let kk = var(k1, k2);
                debug!("3xkk - SE V{:x} ({:x}) {:x}", x, vx, kk);

                if vx == kk {
                    PC::Skip
                } else {
                    PC::Next
                }
            }
            // 4xkk - SNE Vx, byte
            (0x4, x, k1, k2) => {
                let vx = self.registers[x as usize];
                let kk = var(k1, k2);
                debug!("4xkk - SNE V{:x} ({:x}) {:x}", x, vx, kk);

                if vx != kk {
                    PC::Skip
                } else {
                    PC::Next
                }
            }
            // 5xy0 - SE Vx, Vy
            (0x5, x, y, 0x0) => {
                let vx = self.registers[x as usize];
                let vy = self.registers[y as usize];
                debug!("5xy0 - SE V{:x} ({:x}) V{:x} ({:x})", x, vx, y, vy);

                if vx == vy {
                    PC::Skip
                } else {
                    PC::Next
                }
            }
            // 6xkk - LD Vx, byte
            (0x6, x, k1, k2) => {
                let kk = var(k1, k2);
                debug!(
                    "6xkk - LD V{:x} ({:x}) {:x}",
                    x, self.registers[x as usize], kk
                );

                self.registers[x as usize] = kk;
                PC::Next
            }
            // 7xkk - ADD Vx, byte
            (0x7, x, k1, k2) => {
                let kk = var(k1, k2);
                debug!(
                    "7xkk - ADD V{:x} ({:x}) {:x}",
                    x, self.registers[x as usize], kk
                );

                self.registers[x as usize] = self.registers[x as usize].overflowing_add(kk).0;
                PC::Next
            }
            // 8xy0 - LD Vx, Vy
            (0x8, x, y, 0x0) => {
                debug!(
                    "8xy0 - LD V{:x} ({:x}), V{:x} ({:x})",
                    x, self.registers[x as usize], y, self.registers[y as usize]
                );

                self.registers[x as usize] = self.registers[y as usize];
                PC::Next
            }
            // 8xy1 - OR Vx, Vy
            (0x8, x, y, 0x1) => {
                debug!(
                    "8xy1 - OR V{:x} ({:x}), V{:x} ({:x})",
                    x, self.registers[x as usize], y, self.registers[y as usize]
                );
                self.registers[x as usize] |= self.registers[y as usize];
                PC::Next
            }
            // 8xy2 - AND Vx, Vy
            (0x8, x, y, 0x2) => {
                debug!(
                    "8xy2 - AND V{:x} ({:x}), V{:x} ({:x})",
                    x, self.registers[x as usize], y, self.registers[y as usize]
                );

                self.registers[x as usize] &= self.registers[y as usize];
                PC::Next
            }
            // 8xy3 - XOR Vx, Vy
            (0x8, x, y, 0x3) => {
                debug!(
                    "8xy3 - XOR V{:x} ({:x}), V{:x} ({:x})",
                    x, self.registers[x as usize], y, self.registers[y as usize]
                );

                self.registers[x as usize] ^= self.registers[y as usize];
                PC::Next
            }
            // 8xy4 - ADD Vx, Vy
            (0x8, x, y, 0x4) => {
                debug!(
                    "8xy4 - ADD V{:x} ({:x}), V{:x} ({:x})",
                    x, self.registers[x as usize], y, self.registers[y as usize]
                );

                let s = (self.registers[x as usize] as u16) + (self.registers[y as usize] as u16);

                if s > 0xFF {
                    self.registers[0xF] = 1;
                } else {
                    self.registers[0xF] = 0;
                }
                self.registers[x as usize] = (s & 0xFF) as u8;

                PC::Next
            }
            // 8xy5 - SUB Vx, Vy
            (0x8, x, y, 0x5) => {
                debug!(
                    "8xy5 - SUB V{:x} ({:x}), V{:x} ({:x})",
                    x, self.registers[x as usize], y, self.registers[y as usize]
                );

                let (d, flag) = (self.registers[x as usize] as u16)
                    .overflowing_sub(self.registers[y as usize] as u16);

                if !flag {
                    self.registers[0xF] = 1;
                } else {
                    self.registers[0xF] = 0;
                }
                self.registers[x as usize] = d as u8;

                PC::Next
            }
            // 8xy6 - SHR Vx
            (0x8, x, _y, 0x6) => {
                debug!("8xy6 - SHR V{:x} ({:x})", x, self.registers[x as usize]);

                self.registers[0xF] = self.registers[x as usize] & 0x1;
                self.registers[x as usize] /= 2;

                PC::Next
            }
            // 8xy7 - SUBN Vx, Vy
            (0x8, x, y, 0x7) => {
                let vx = self.registers[x as usize];
                let vy = self.registers[y as usize];
                debug!("8xy7 - SUBN V{:x} ({:x}), V{:x} ({:x})", x, vx, y, vy);

                let (d, flag) = vy.overflowing_sub(vx);

                if !flag {
                    self.registers[0xF] = 1;
                } else {
                    self.registers[0xF] = 0;
                }
                self.registers[x as usize] = d;

                PC::Next
            }
            // 8xyE - SHL VX {, Vy}
            (0x8, x, _y, 0xE) => {
                debug!(
                    "8xyE - SHL V{:x} ({:x}) {{, Vy}}",
                    x, self.registers[x as usize]
                );

                self.registers[0xF] = self.registers[x as usize] >> 7;
                self.registers[x as usize] = self.registers[x as usize].overflowing_mul(2).0;

                PC::Next
            }
            // 9xy0 - SNE Vx, Vy
            (0x9, x, y, 0x0) => {
                let vx = self.registers[x as usize];
                let vy = self.registers[y as usize];
                debug!("9xy0 - SNE V{:x} ({:x}), V{:x} ({:x})", x, vx, y, vy);

                if vx != vy {
                    PC::Skip
                } else {
                    PC::Next
                }
            }
            // Annn - LD I, addr
            (0xA, n1, n2, n3) => {
                let nnn = nnn(n1, n2, n3) as usize;
                debug!("Annn - LD {:x}, {:x}", self.index, nnn);

                self.index = nnn;
                PC::Next
            }
            // Bnnn - LD V0, addr
            (0xB, n1, n2, n3) => {
                let nnn = nnn(n1, n2, n3) as usize;
                debug!("Bnnn - LD V0, {:x}", nnn);

                PC::Jump(nnn)
            }
            // Cxkk - RND Vx, byte
            (0xC, x, k1, k2) => {
                let kk = var(k1, k2);
                debug!(
                    "Cxkk - RND V{:x} ({:x}), {:x}",
                    x, self.registers[x as usize], kk
                );

                self.registers[x as usize] = self.gen_random() & kk;
                PC::Next
            }
            // Dxyn - DRW Vx, Vy, nibble
            (0xD, x, y, n) => {
                let vx = self.registers[x as usize];
                let vy = self.registers[y as usize];
                debug!(
                    "Dxyn - DRW V{:x} ({:x}), V{:x} ({:x}), {:x}",
                    x, vx, y, vy, n
                );

                let mem_start = self.index as usize;
                let bytes = &self.memory[mem_start..(mem_start + n as usize)].to_vec();

                self.registers[0xF] = self.display.draw(vx as usize, vy as usize, bytes);
                PC::Next
            }
            // Ex9E - SKP Vx
            (0xE, x, 0x9, 0xE) => {
                let vx = self.registers[x as usize];
                debug!("Ex9E - SKP V{:x} ({:x})", x, vx);

                if self.keypad[vx as usize] == 1 {
                    PC::Skip
                } else {
                    PC::Next
                }
            }
            // ExA1 - SKNP Vx
            (0xE, x, 0xA, 0x1) => {
                let vx = self.registers[x as usize];
                debug!("ExA1 - SKNP V{:x} ({:x})", x, vx);

                if self.keypad[vx as usize] != 1 {
                    PC::Skip
                } else {
                    PC::Next
                }
            }
            // Fx07 - LD Vx, DT
            (0xF, x, 0x0, 0x7) => {
                let vx = self.registers[x as usize];
                debug!("Fx07 - LD V{:x} ({:x}), DT", x, vx);

                self.registers[x as usize] = self.delay_timer;
                PC::Next
            }
            // Fx0A - LD Vx, K
            (0xF, x, 0x0, 0xA) => {
                let vx = self.registers[x as usize];
                debug!("Fx0A - LD V{:x} ({:x}), K", x, vx);

                let mut pressed = false;

                for (n, &i) in self.keypad.iter().enumerate() {
                    if i == 1 {
                        self.registers[x as usize] = n as u8;
                        pressed = true;
                        break;
                    }
                }

                if pressed {
                    PC::Next
                } else {
                    PC::Jump(self.pc)
                }
            }
            // Fx15 - LD DT, Vx
            (0xF, x, 0x1, 0x5) => {
                let vx = self.registers[x as usize];
                debug!("Fx15 - LD DT, V{:x} ({:x})", x, vx);

                self.delay_timer = vx;

                PC::Next
            }
            // Fx18 - LD ST, Vx
            (0xF, x, 0x1, 0x8) => {
                let vx = self.registers[x as usize];
                debug!("Fx18 - LD ST, V{:x} ({:x})", x, vx);

                self.sound_timer = self.registers[x as usize];

                PC::Next
            }
            // Fx1E - ADD I, Vx
            (0xF, x, 0x1, 0xE) => {
                let vx = self.registers[x as usize];
                debug!("Fx1E - ADD {:x}, V{:x} ({:x})", self.index, x, vx);

                self.index += vx as usize;

                PC::Next
            }
            // Fx29 - LD F, Vx
            (0xF, x, 0x2, 0x9) => {
                let vx = self.registers[x as usize];
                debug!("Fx29 - LD F, V{:x} ({:x})", x, vx);

                self.index =
                    Self::FONTSET_START_ADDRESS + (5 * self.registers[x as usize] as usize);

                PC::Next
            }
            // Fx33 - LD B, Vx
            (0xF, x, 0x3, 0x3) => {
                let vx = self.registers[x as usize];
                debug!("Fx33 - LD B, V{:x} ({:x})", x, vx);

                self.memory[self.index] = (vx / 100) as u8 % 10;
                self.memory[self.index + 1] = (vx / 10) as u8 % 10;
                self.memory[self.index + 2] = vx % 10;

                PC::Next
            }
            // Fx55 - LD [I], Vx
            (0xF, x, 0x5, 0x5) => {
                debug!("Fx55 - LD [I], V{:x}", x);

                for n in 0..(x as usize + 1) {
                    self.memory[self.index + n as usize] = self.registers[n as usize];
                }

                PC::Next
            }
            // Fx65 - LD Vx, [I]
            (0xF, x, 0x6, 0x5) => {
                debug!("Fx65 - LD V{:x}, [I]", x);

                for n in 0..(x as usize + 1) {
                    self.registers[n] = self.memory[self.index as usize + n];
                }

                PC::Next
            }
            _ => panic!("Unknown instruction: {:x}{:x}{:x}{:x}", o1, o2, o3, o4),
        };

        match pc_change {
            PC::Next => self.pc += 2,
            PC::Skip => self.pc += 4,
            PC::Jump(v) => self.pc = v,
        }
    }

    pub fn cycle(&mut self) {
        let opcode = ((self.memory[self.pc] as u16) << 8) | self.memory[self.pc + 1] as u16;
        self.process_instruction(opcode);

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn press_key(&mut self, idx: usize) {
        self.keypad[idx] = 1;
    }

    pub fn lift_key(&mut self, idx: usize) {
        self.keypad[idx] = 0;
    }

    pub fn get_video(&self) -> &[u32; Display::SIZE] {
        self.display.view()
    }

    pub fn is_dirty(&self) -> bool {
        self.display.is_dirty()
    }

    pub fn set_clean(&mut self) {
        self.display.set_clean()
    }
}
