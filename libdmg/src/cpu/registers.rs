use super::decoder::{Operand16, Register};

#[derive(Default)]
pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }

    pub fn read(&self, register: Register) -> u8 {
        match register {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
        }
    }

    pub fn read_double(&self, source: Operand16) -> u16 {
        match source {
            Operand16::Bc => u16::from_le_bytes([self.c, self.b]),
            Operand16::De => u16::from_le_bytes([self.e, self.d]),
            Operand16::Hl => u16::from_le_bytes([self.l, self.h]),
            Operand16::Sp => self.sp,
            Operand16::Af => u16::from_le_bytes([self.f, self.a]),
        }
    }

    pub fn bc(&self) -> u16 {
        self.read_double(Operand16::Bc)
    }

    pub fn de(&self) -> u16 {
        self.read_double(Operand16::De)
    }

    pub fn hl(&self) -> u16 {
        self.read_double(Operand16::Hl)
    }

    pub fn write(&mut self, register: Register, value: u8) {
        match register {
            Register::A => self.a = value,
            Register::B => self.b = value,
            Register::C => self.c = value,
            Register::D => self.d = value,
            Register::E => self.e = value,
            Register::H => self.h = value,
            Register::L => self.l = value,
        }
    }

    pub fn write_double(&mut self, destination: Operand16, val: u16) {
        let [low, high] = val.to_le_bytes();

        match destination {
            Operand16::Bc => {
                self.b = high;
                self.c = low;
            }
            Operand16::De => {
                self.d = high;
                self.e = low;
            }
            Operand16::Hl => {
                self.h = high;
                self.l = low;
            }
            Operand16::Sp => {
                self.sp = val;
            }
            Operand16::Af => {
                self.a = high;
                self.f = low;
            }
        }
    }

    pub fn set_hl(&mut self, val: u16) {
        self.write_double(Operand16::Hl, val)
    }

    fn set_flag(&mut self, bit: u8, val: bool) {
        let mask = 1 << bit;

        if val {
            self.f |= mask;
        } else {
            self.f &= !mask;
        }
    }

    pub fn set_zero(&mut self, zero: bool) {
        self.set_flag(7, zero);
    }

    pub fn set_bcd_n(&mut self, bcd_n: bool) {
        self.set_flag(6, bcd_n);
    }

    pub fn set_bcd_h(&mut self, bcd_h: bool) {
        self.set_flag(5, bcd_h);
    }

    pub fn set_carry(&mut self, carry: bool) {
        self.set_flag(4, carry)
    }

    fn flag(&self, bit: u8) -> bool {
        let mask = 1 << bit;

        (self.f & mask) != 0
    }

    pub fn zero(&self) -> bool {
        self.flag(7)
    }

    pub fn bcd_n(&self) -> bool {
        self.flag(6)
    }

    pub fn bcd_h(&self) -> bool {
        self.flag(5)
    }

    pub fn carry(&self) -> bool {
        self.flag(4)
    }
}
