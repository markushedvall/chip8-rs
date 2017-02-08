use std::io::Read;

use std::error::Error;
use std::fmt;
use std::io;
use std::slice::Chunks;

pub trait Vm {
    fn step(&mut self, time: f64) -> Result<(), InstructionError>;
    fn load_rom<T: Read>(&mut self, input: &mut T) -> io::Result<()>;
    fn pixels<'a>(&'a self) -> Chunks<'a, bool>;
    fn press_key(&mut self, key: Key);
    fn release_key(&mut self, key: Key);
}

#[derive(Clone, Copy, Debug)]
pub enum Key {
    D0 = 0x0,
    D1 = 0x1,
    D2 = 0x2,
    D3 = 0x3,
    D4 = 0x4,
    D5 = 0x5,
    D6 = 0x6,
    D7 = 0x7,
    D8 = 0x8,
    D9 = 0x9,
    A  = 0xA,
    B  = 0xB,
    C  = 0xC,
    D  = 0xD,
    E  = 0xE,
    F  = 0xF,
}

#[derive(Clone, Copy, Debug)]
pub enum InstructionError {
    Illegal,
    Unsupported,
}

impl fmt::Display for InstructionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InstructionError::Illegal => write!(f, ""),
            InstructionError::Unsupported => write!(f, ""),
        }
    }
}

impl Error for InstructionError {
    fn description(&self) -> &str {
        match *self {
            InstructionError::Illegal => "",
            InstructionError::Unsupported => "",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}
