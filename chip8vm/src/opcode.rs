#[derive(Clone, Copy, Debug)]
pub struct Opcode {
    bits: u16,
}

impl Opcode {
    pub fn new(bits: u16) -> Opcode{
        Opcode{ bits: bits }
    }

    pub fn bits(&self) -> u16 {
        self.bits
    }

    // A 12-bit value, the lowest 12 bits of the instruction
    pub fn addr(&self) -> u16 {
        self.bits & 0x0FFF
    }

    // A 4-bit value, the lowest 4 bits of the instruction
    pub fn nibble(&self) -> u8 {
        (self.bits & 0x000F) as u8
    }

    //  A 4-bit value, the lower 4 bits of the high byte of the instruction
    pub fn x(&self) -> u8 {
        ((self.bits & 0x0F00) >> 8) as u8
    }

    // A 4-bit value, the upper 4 bits of the low byte of the instruction
    pub fn y(self) -> u8 {
        ((self.bits & 0x00F0) >> 4) as u8
    }

    // An 8-bit value, the lowest 8 bits of the instruction
    pub fn byte(self) -> u8 {
        (self.bits & 0x00FF) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_returns_full_opcode() {
        let opcode = Opcode::new(0xF1E2);
        assert_eq!(opcode.bits(), 0xF1E2);
    }

    #[test]
    fn addr_returns_lowest_12_bits() {
        let opcode = Opcode::new(0xF1E2);
        assert_eq!(opcode.addr(), 0x01E2);
    }

    #[test]
    fn nibble_returns_lowest_4_bits() {
        let opcode = Opcode::new(0xF1E2);
        assert_eq!(opcode.nibble(), 0x2);
    }

    #[test]
    fn x_returns_lower_4_bits_of_the_high_byte() {
        let opcode = Opcode::new(0xF1E2);
        assert_eq!(opcode.x(), 0x1);
    }

    #[test]
    fn y_returns_higher_4_bits_of_the_low_byte() {
        let opcode = Opcode::new(0xF1E2);
        assert_eq!(opcode.y(), 0xE);
    }

    #[test]
    fn byte_returns_lowest_8_bits() {
        let opcode = Opcode::new(0xF1E2);
        assert_eq!(opcode.byte(), 0xE2);
    }
}
