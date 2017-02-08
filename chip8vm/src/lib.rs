extern crate chip8core;
extern crate rand;
mod opcode;

use chip8core::{ Vm, InstructionError, Key };
use opcode::Opcode;
use std::default::Default;
use std::io::Read;
use std::io;
use std::slice::Chunks;

const PROGRAM_START: usize = 0x200;

const GFX_W: usize = 64;
const GFX_H: usize = 32;

const F: usize = 0xF;

const CLOCK_FREQ: i32 = 540;
const CLOCK_PERIOD: f64 = 1.0 / CLOCK_FREQ as f64;
const TICK_FREQ: i32 = 60;
const TICK_PERIOD: f64 = 1.0 / TICK_FREQ as f64;

const FONT: [u8; 80] = [
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
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Cpu {
    mem: [u8; 4096],
    v: [u8; 16],
    i: u16,
    pc: u16,
    gfx: [bool; 64 * 32],
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; 16],
    sp: u16,
    keys: [bool; 16],

    clock_accumulator: f64,
    tick_accumulator: f64,
    awaited_key: Option<u8>,
}

impl Default for Cpu {
    fn default() -> Cpu {
        let mut cpu = Cpu {
            mem: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: PROGRAM_START as u16,
            gfx: [false; 64 * 32],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            sp: 0,
            keys: [false; 16],
            clock_accumulator: 0.0,
            tick_accumulator: 0.0,
            awaited_key: None,
        };

        (&mut cpu.mem[..FONT.len()]).copy_from_slice(&FONT);

        cpu
    }
}

impl Vm for Cpu {
    fn step(&mut self, time: f64) -> Result<(), InstructionError> {
        self.clock_accumulator += time;

        while self.clock_accumulator > CLOCK_PERIOD {
            self.clock_accumulator -= CLOCK_PERIOD;

            self.tick_accumulator += CLOCK_PERIOD;
            if self.tick_accumulator > TICK_PERIOD {
                self.tick_accumulator -= TICK_PERIOD;
                self.tick_timers();
            }

            if !self.awaited_key.is_some() {
                try!(self.cycle());
            }
        }

        Ok(())
    }

    fn load_rom<T: Read>(&mut self, reader: &mut T) -> io::Result<()> {
        let mut rom_counter = PROGRAM_START;

        loop {
            let mem_rom = &mut self.mem[rom_counter..];
            if mem_rom.len() == 0 {
                // All rom memory used up, check if eof file has been reached
                let mut eof_check = [0u8, 1];
                if try!(reader.read(&mut eof_check)) != 0 {
                    return Err(io::Error::new(io::ErrorKind::InvalidData,
                                              "ROM exceeds maximum size"));
                }
                break;
            }

            let n = try!(reader.read(mem_rom));
            if n == 0 { break;}
            rom_counter += n;
        }
        Ok(())
    }

    fn pixels<'a>(&'a self) -> Chunks<'a, bool> {
        self.gfx.chunks(GFX_W)
    }

    fn press_key(&mut self, key: Key) {
        self.keys[key as usize] = true;
        if let Some(x) = self.awaited_key {
            self.v[x as usize] = key as u8;
            self.awaited_key = None;
        }
    }

    fn release_key(&mut self, key: Key) {
        self.keys[key as usize] = false;
    }
}

impl Cpu {
    pub fn new() -> Cpu {
        Default::default()
    }

    fn tick_timers(&mut self) {
        if self.delay_timer > 0 { self.delay_timer -= 1; }
        if self.sound_timer > 0 { self.sound_timer -= 1; }
    }

    fn cycle(&mut self) -> Result<(), InstructionError> {
        let opcode = get_opcode(&mut self.mem, self.pc);
        try!(self.exec_opcode(opcode));
        Ok(())
    }

    fn exec_opcode(&mut self, opcode: Opcode) -> Result<(), InstructionError> {
        let x = opcode.x();
        let y = opcode.y();
        let addr = opcode.addr();
        let byte = opcode.byte();
        let nibble = opcode.nibble();

        self.pc += 2;

        match opcode.bits() & 0xF000 {
            0x0000 => match opcode.bits() & 0x0FFF {
                0x00E0 => self.clear(),
                0x00EE => self.ret(),
                _      => return Err(InstructionError::Unsupported), // Only for RCA 1802 hw
            },
            0x1000 => self.jump(addr),
            0x2000 => self.call(addr),
            0x3000 => self.skip_eq_byte(x, byte),
            0x4000 => self.skip_neq_byte(x, byte),
            0x5000 => self.skip_eq(x, y),
            0x6000 => self.set_byte(x, byte),
            0x7000 => self.add_byte(x, byte),
            0x8000 => match opcode.bits() & 0x000F {
                0x0000 => self.set(x, y),
                0x0001 => self.or(x, y),
                0x0002 => self.and(x, y),
                0x0003 => self.xor(x, y),
                0x0004 => self.add(x, y),
                0x0005 => self.sub(x, y),
                0x0006 => self.shift_right(x),
                0x0007 => self.subn(x, y),
                0x000E => self.shift_left(x),
                _      => return Err(InstructionError::Illegal),
            },
            0x9000 => self.skip_neq(x, y),
            0xA000 => self.set_i(addr),
            0xB000 => self.jump_v0(addr),
            0xC000 => self.rand(x, byte),
            0xD000 => self.draw(x, y, nibble),
            0xE000 => match opcode.bits() & 0x00FF {
                0x009E => self.skip_pressed(x),
                0x00A1 => self.skip_not_pressed(x),
                _      => return Err(InstructionError::Illegal),
            },
            0xF000 => match opcode.bits() & 0x00FF {
                0x0007 => self.get_delay_timer(x),
                0x000A => self.await_key_press(x),
                0x0015 => self.set_delay_timer(x),
                0x0018 => self.set_sound_timer(x),
                0x001E => self.i_add(x),
                0x0029 => self.set_char(x),
                0x0033 => self.store_bcd(x),
                0x0055 => self.store_regs(x),
                0x0065 => self.load_regs(x),
                _      => return Err(InstructionError::Illegal),
            },
            _      => return Err(InstructionError::Illegal),
        }
        Ok(())
    }

    fn clear(&mut self) {
        for pixel in self.gfx.iter_mut() {
            *pixel = false;
        }
    }

    fn ret(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    fn jump(&mut self, addr: u16) {
        self.pc = addr;
    }

    fn call(&mut self, addr: u16) {
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = addr;
    }

    fn skip_eq_byte(&mut self, x: u8, byte: u8) {
        let vx = self.v[x as usize];
        if vx == byte {
            self.pc += 2;
        }
    }

    fn skip_neq_byte(&mut self, x: u8, byte: u8) {
        let vx = self.v[x as usize];
        if vx != byte {
            self.pc += 2;
        }
    }

    fn skip_eq(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        if vx == vy {
            self.pc += 2;
        }
    }

    fn set_byte(&mut self, x: u8, byte: u8) {
        self.v[x as usize] = byte;
    }

    fn add_byte(&mut self, x: u8, byte: u8) {
        self.v[x as usize] = self.v[x as usize].wrapping_add(byte);
    }

    fn set(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[y as usize];
    }

    fn or(&mut self, x: u8, y: u8) {
        self.v[x as usize] |= self.v[y as usize];
    }

    fn and(&mut self, x: u8, y: u8) {
        self.v[x as usize] &= self.v[y as usize];
    }

    fn xor(&mut self, x: u8, y: u8) {
        self.v[x as usize] ^= self.v[y as usize];
    }

    fn add(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        self.v[x as usize] = vx.wrapping_add(vy);

        self.v[F] = ((vx as u16 + vy as u16) > 0xff) as u8;
    }

    fn sub(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        self.v[x as usize] = vx.wrapping_sub(vy);

        self.v[F] = (vx > vy) as u8;
    }

    fn shift_right(&mut self, x: u8) {
        self.v[F] = self.v[x as usize] & 0b1;
        self.v[x as usize] >>= 1;
    }

    fn subn(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        self.v[x as usize] = vy.wrapping_sub(vx);

        self.v[F] = (vy > vx) as u8;
    }

    fn shift_left(&mut self, x: u8) {
        self.v[F] = self.v[x as usize] >> 7;
        self.v[x as usize] <<= 1;
    }

    fn skip_neq(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        if vx != vy {
            self.pc += 2;
        }
    }

    fn set_i(&mut self, addr: u16) {
        self.i = addr;
    }

    fn jump_v0(&mut self, addr: u16) {
        self.pc = (addr + self.v[0] as u16) & 0x0FFF;
    }

    fn rand(&mut self, x: u8, byte: u8) {
        self.v[x as usize] = rand::random::<u8>() & byte;
    }

    fn draw(&mut self, x: u8, y: u8, nibble: u8) {
        let vx = self.v[x as usize] as usize;
        let vy = self.v[y as usize] as usize;
        let n = nibble as usize;
        let i = self.i as usize;

        let spr = &self.mem[i..(i + n)];
        self.v[F] = 0;

        for (spr_y, byte) in spr.iter().enumerate() {
            let gfx_y = (vy + spr_y) % GFX_H;
            for spr_x in 0..8 as usize {
                let mask = 0b1000_0000 >> spr_x;
                let is_sprite_pixel = (byte & mask) != 0;

                let gfx_x = (vx + spr_x) % GFX_W;
                let idx = (gfx_y * GFX_W) + gfx_x;

                self.gfx[idx] ^= is_sprite_pixel;
                self.v[F] |= (self.gfx[idx] == false && is_sprite_pixel == true) as u8;
            }
        }
    }

    fn skip_pressed(&mut self, x: u8) {
        let vx = self.v[x as usize] as usize;
        if self.keys[vx] {
            self.pc += 2;
        }
    }

    fn skip_not_pressed(&mut self, x: u8) {
        let vx = self.v[x as usize] as usize;
        if !self.keys[vx] {
            self.pc += 2;
        }
    }

    fn get_delay_timer(&mut self, x: u8) {
        self.v[x as usize] = self.delay_timer;
    }

    fn await_key_press(&mut self, x: u8) {
        self.awaited_key = Some(x);
    }

    fn set_delay_timer(&mut self, x: u8) {
         self.delay_timer = self.v[x as usize];
    }

    fn set_sound_timer(&mut self, x: u8) {
         self.sound_timer = self.v[x as usize];
    }

    fn i_add(&mut self, x: u8) {
        self.i += self.v[x as usize] as u16;
    }

    fn set_char(&mut self, x: u8) {
        let vx = self.v[x as usize] as usize;
        self.i = FONT[vx * 5] as u16;
    }

    fn store_bcd(&mut self, x: u8) {
        let vx = self.v[x as usize];
        let i = self.i as usize;

        self.mem[i] = vx / 100;
        self.mem[i + 1] = (vx / 10) % 10;
        self.mem[i + 2] = (vx / 100) % 10;
    }

    fn store_regs(&mut self, x: u8) {
        for i in 0..(x + 1) as usize {
            self.mem[(self.i as usize) + i] = self.v[i];
        }
    }

    fn load_regs(&mut self, x: u8) {
        for i in 0..(x + 1) as usize {
             self.v[i] = self.mem[(self.i as usize) + i];
        }
    }
}

fn get_opcode(mem: &[u8], pc: u16) -> Opcode {
    Opcode::new((mem[pc as usize] as u16) << 8 | (mem[(pc as usize) + 1] as u16))
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::FONT;
    use chip8core::Vm;
    use std::io::Cursor;
    use opcode::Opcode;

    #[test]
    fn new_intializes_cpu() {
        let cpu = Cpu::new();

        assert_eq!(cpu.i, 0);
        assert_eq!(cpu.pc, 0x200);
        assert_eq!(cpu.delay_timer, 0);
        assert_eq!(cpu.sound_timer, 0);
        assert_eq!(cpu.sp, 0);

        assert!(cpu.v.iter().all(|&x| x == 0));
        assert!(cpu.gfx.iter().all(|&x| x == false));
        assert!(cpu.stack.iter().all(|&x| x == 0));
        assert!(cpu.keys.iter().all(|&x| x == false));

        // Check that the fontset was set into memory
        for i in 0..80 {
            assert_eq!(FONT[i], cpu.mem[i]);
        }

        // Check that the rest of the memory is zero
        let m = &cpu.mem[80..];
        let all_zero = m.iter().all(|&x| x == 0);
        assert!(all_zero);

        assert_eq!(cpu.clock_accumulator, 0.0);
        assert_eq!(cpu.tick_accumulator, 0.0);

        assert_eq!(awaited_key, None);
    }

    #[test]
    fn load_rom_reads_into_memory() {
        let rom = [0xF5, 0xC2, 0xF5, 0xC2];
        let mut rom_reader = Cursor::new(&rom);

        let mut cpu = Cpu::new();
        cpu.load_rom(&mut rom_reader);

        for i in 0..rom.len() {
            assert_eq!(cpu.mem[0x200 + i], rom[i]);
        }
    }

    #[test]
    fn load_rom_with_exactly_max_size_returns_ok() {
        let max_rom_size = 4096 - 0x200;
        let mut rom_reader = Cursor::new(vec![1u8; max_rom_size]);

        let mut cpu = Cpu::new();
        cpu.load_rom(&mut rom_reader);
    }

    #[test]
    fn clear_00e0() {
        let mut cpu = Cpu::new();
        cpu.gfx.iter_mut().map(|p| *p = true);

        cpu.exec_opcode(Opcode::new(0x00E0)).unwrap();

        let all_false = cpu.gfx.iter().all(|&p| p == false);
        assert!(all_false);
        assert_eq!(cpu.pc, 0x200 + 2);
    }

    #[test]
    pub fn ret_00ee() {
        let mut cpu = Cpu::new();

        cpu.pc = 0x0BBB;
        cpu.stack[0] = 0x0AAA;
        cpu.sp = 1;

        cpu.exec_opcode(Opcode::new(0x00EE)).unwrap();

        assert_eq!(cpu.sp, 0);
        assert_eq!(cpu.pc, 0x0AAA);
    }

    #[test]
    fn jump_1nnn() {
        let mut cpu = Cpu::new();

        cpu.exec_opcode(Opcode::new(0x1ABC)).unwrap();

        assert_eq!(cpu.pc, 0x0ABC);
    }

    #[test]
    fn call_2nnn() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x0AAA;

        cpu.exec_opcode(Opcode::new(0x2ABC)).unwrap();

        assert_eq!(cpu.sp, 1);
        assert_eq!(cpu.pc, 0x0ABC);
        assert_eq!(cpu.stack[0], 0x0AAA + 2);
    }

    #[test]
    fn skip_eq_byte_3xnn_equals() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;

        cpu.exec_opcode(Opcode::new(0x3A25)).unwrap();

        assert_eq!(cpu.pc, 0x0200 + 4);
    }

    #[test]
    fn skip_eq_byte_3xnn_not_equal() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;

        cpu.exec_opcode(Opcode::new(0x3A26)).unwrap();

        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn skip_neq_byte_4xnn_not_equal() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;

        cpu.exec_opcode(Opcode::new(0x4A26)).unwrap();

        assert_eq!(cpu.pc, 0x0200 + 4);
    }

    #[test]
    fn skip_neq_byte_4xnn_equals() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;

        cpu.exec_opcode(Opcode::new(0x4A25)).unwrap();

        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn skip_eq_5xy0_equals() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;
        cpu.v[0xB] = 0x25;

        cpu.exec_opcode(Opcode::new(0x5AB0)).unwrap();

        assert_eq!(cpu.pc, 0x0200 + 4);
    }

    #[test]
    fn skip_eq_5xy0_not_equal() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;
        cpu.v[0xB] = 0x26;

        cpu.exec_opcode(Opcode::new(0x5AB0)).unwrap();

        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn set_byte_6xnn() {
        let mut cpu = Cpu::new();

        cpu.exec_opcode(Opcode::new(0x6A25)).unwrap();

        assert_eq!(cpu.v[0xA], 0x25);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn add_byte_7xnn() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x13;

        cpu.exec_opcode(Opcode::new(0x7A25)).unwrap();

        assert_eq!(cpu.v[0xA], 0x13 + 0x25);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn set_8xy0() {
        let mut cpu = Cpu::new();
        cpu.v[0xB] = 0x25;

        cpu.exec_opcode(Opcode::new(0x8AB0)).unwrap();

        assert_eq!(cpu.v[0xA], cpu.v[0xB]);
        assert_eq!(cpu.v[0xB], 0x25);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn or_8xy1() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;
        cpu.v[0xB] = 0x26;

        cpu.exec_opcode(Opcode::new(0x8AB1)).unwrap();

        assert_eq!(cpu.v[0xA], 0x25 | 0x26);
        assert_eq!(cpu.v[0xB], 0x26);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn and_8xy2() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;
        cpu.v[0xB] = 0x26;

        cpu.exec_opcode(Opcode::new(0x8AB2)).unwrap();

        assert_eq!(cpu.v[0xA], 0x25 & 0x26);
        assert_eq!(cpu.v[0xB], 0x26);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn xor_8xy3() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0x25;
        cpu.v[0xB] = 0x26;

        cpu.exec_opcode(Opcode::new(0x8AB3)).unwrap();

        assert_eq!(cpu.v[0xA], 0x25 ^ 0x26);
        assert_eq!(cpu.v[0xB], 0x26);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn add_8xy4() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0xF5;
        cpu.v[0xB] = 0x05;

        cpu.exec_opcode(Opcode::new(0x8AB4)).unwrap();

        assert_eq!(cpu.v[0xA], 0xFA);
        assert_eq!(cpu.v[0xB], 0x05);
        assert_eq!(cpu.v[0xF], 0);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn add_8xy4_carry() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0xFF;
        cpu.v[0xB] = 0x05;

        cpu.exec_opcode(Opcode::new(0x8AB4)).unwrap();

        assert_eq!(cpu.v[0xA], 0x04);
        assert_eq!(cpu.v[0xB], 0x05);
        assert_eq!(cpu.v[0xF], 1);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    #[test]
    fn sub_8xy5_no_borrow() {
        let mut cpu = Cpu::new();
        cpu.v[0xA] = 0xA5;
        cpu.v[0xB] = 0xA3;

        cpu.exec_opcode(Opcode::new(0x8AB5)).unwrap();

        assert_eq!(cpu.v[0xA], 0x02);
        assert_eq!(cpu.v[0xB], 0xA3);
        assert_eq!(cpu.v[0xF], 1);
        assert_eq!(cpu.pc, 0x0200 + 2);
    }

    // #[test]
    // fn sub_8xy5_borrow() {
    // }


    /*

    fn sub(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        self.v[x as usize] = vx.wrapping_sub(vy);

        self.v[F] = (vx > vy) as u8;
    }

    fn shift_right(&mut self, x: u8) {
        self.v[F] = self.v[x as usize] & 0b1;
        self.v[x as usize] >>= 1;
    }

    fn subn(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        self.v[x as usize] = vy.wrapping_sub(vx);

        self.v[F] = (vy > vx) as u8;
    }

    fn shift_left(&mut self, x: u8) {
        self.v[F] = self.v[x as usize] >> 7;
        self.v[x as usize] <<= 1;
    }

    fn skip_neq(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        if vx != vy {
            self.pc += 2;
        }
    }

*/

}
