extern crate chip8vm;
extern crate chip8ui;

use chip8vm::Cpu;
use chip8ui::Runner;

fn main() {
    let mut cpu = Cpu::new();
    Runner::run(&mut cpu).unwrap();
}
