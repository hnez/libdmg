use std::ops::{BitAnd, BitOr};

#[derive(Debug, Copy, Clone)]
pub enum Interrupt {
    VBlank,
    Lcd,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    pub const fn as_mask(self) -> InterruptMask {
        InterruptMask(1 << (self as u8))
    }

    pub const fn vector_address(self) -> u16 {
        match self {
            Self::VBlank => 0x0040,
            Self::Lcd => 0x0048,
            Self::Timer => 0x0050,
            Self::Serial => 0x0058,
            Self::Joypad => 0x0060,
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct InterruptMask(u8);

impl InterruptMask {
    pub fn highest_priority(&self) -> Option<Interrupt> {
        if self.is_set(Interrupt::VBlank) {
            Some(Interrupt::VBlank)
        } else if self.is_set(Interrupt::Lcd) {
            Some(Interrupt::Lcd)
        } else if self.is_set(Interrupt::Timer) {
            Some(Interrupt::Timer)
        } else if self.is_set(Interrupt::Serial) {
            Some(Interrupt::Serial)
        } else if self.is_set(Interrupt::Joypad) {
            Some(Interrupt::Joypad)
        } else {
            None
        }
    }

    pub fn set(&mut self, int: Interrupt) {
        self.0 |= int.as_mask().0
    }

    pub fn clear(&mut self, int: Interrupt) {
        self.0 &= !int.as_mask().0
    }

    pub fn is_set(self, int: Interrupt) -> bool {
        (self.0 & int.as_mask().0) != 0
    }
}

impl From<u8> for InterruptMask {
    fn from(value: u8) -> Self {
        Self(value & 0b0001_1111)
    }
}

impl From<InterruptMask> for u8 {
    fn from(value: InterruptMask) -> Self {
        value.0
    }
}

impl BitAnd<Self> for InterruptMask {
    type Output = Self;

    fn bitand(self, other: InterruptMask) -> Self {
        Self(self.0 & other.0)
    }
}

impl BitOr<Self> for InterruptMask {
    type Output = Self;

    fn bitor(self, other: InterruptMask) -> Self {
        Self(self.0 | other.0)
    }
}

pub trait InterruptSource {
    fn pending(&self, cycle: u64) -> InterruptMask;
    fn set_pending(&mut self, cycle: u64, mask: InterruptMask);
    fn next_pending(&self, cycle: u64) -> u64;
}
