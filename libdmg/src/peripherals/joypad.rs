use super::{Interrupt, InterruptMask, InterruptSource};

#[derive(Clone, Copy)]
pub enum Button {
    A,
    B,
    Select,
    Start,
    Right,
    Left,
    Up,
    Down,
}

impl Button {
    fn as_mask(self) -> u8 {
        1 << (self as u8)
    }
}

#[derive(Clone, Copy)]
pub struct Buttons(u8);

impl Buttons {
    fn new() -> Self {
        Buttons(0xff)
    }

    fn press(mut self, button: Button) -> Self {
        self.0 &= !button.as_mask();
        self
    }

    fn buttons(self) -> u8 {
        self.0 & 0x0f
    }

    fn dpad(self) -> u8 {
        self.0 >> 4
    }
}

impl From<&[Button]> for Buttons {
    fn from(buttons: &[Button]) -> Self {
        buttons
            .iter()
            .fold(Self::new(), |acc, button| acc.press(*button))
    }
}

pub struct Joypad {
    buttons: Buttons,
    select_buttons: bool,
    select_dpad: bool,
    irq_pending: bool,
}

impl Joypad {
    pub(crate) fn new() -> Self {
        Self {
            buttons: Buttons::new(),
            select_buttons: false,
            select_dpad: false,
            irq_pending: false,
        }
    }

    pub(crate) fn buttons(&mut self, buttons: &[Button]) {
        self.buttons = buttons.into();
    }

    pub(crate) fn read(&self) -> u8 {
        match (self.select_buttons, self.select_dpad) {
            (true, false) => 0b0001_0000 | self.buttons.buttons(),
            (false, true) => 0b0010_0000 | self.buttons.dpad(),
            (false, false) => 0b0011_1111,
            (true, true) => 0b0000_1111,
        }
    }

    pub(crate) fn write(&mut self, val: u8) {
        self.select_buttons = (val & 0b0010_0000) == 0;
        self.select_dpad = (val & 0b0001_0000) == 0;
    }
}

impl InterruptSource for Joypad {
    fn pending(&self, _cycle: u64) -> InterruptMask {
        if self.irq_pending {
            Interrupt::Joypad.as_mask()
        } else {
            InterruptMask::default()
        }
    }

    fn set_pending(&mut self, _cycle: u64, mask: InterruptMask) {
        self.irq_pending = mask.is_set(Interrupt::Joypad);
    }

    fn next_pending(&self, _cycle: u64) -> u64 {
        u64::MAX
    }
}
