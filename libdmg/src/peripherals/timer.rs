use super::{Interrupt, InterruptMask, InterruptSource};

#[derive(Clone, Copy)]
enum Clock {
    Div1024,
    Div16,
    Div64,
    Div256,
}

pub struct Timer {
    tma: u8,
    enable: bool,
    clock: Clock,
    irq_pending: bool,
}

impl Timer {
    pub(crate) fn new() -> Self {
        Self {
            tma: 0,
            enable: false,
            clock: Clock::Div1024,
            irq_pending: false,
        }
    }

    pub(crate) fn read(&self, cycle: u64, addr: u16) -> u8 {
        match addr {
            0xff04 => (cycle / 256) as u8,
            0xff05 => unimplemented!("FF05 — TIMA: Timer counter"),
            0xff06 => self.tma,
            0xff07 => {
                let en = self.enable as u8;
                let clk = self.clock as u8;
                en << 2 | clk
            }
            _ => 0,
        }
    }

    pub(crate) fn write(&mut self, _cycle: u64, addr: u16, val: u8) {
        match addr {
            0xff04 => unimplemented!("FF04 — DIV: Divider register"),
            0xff05 => unimplemented!("FF05 — TIMA: Timer counter"),
            0xff06 => {
                self.tma = val;
            }
            0xff07 => {
                self.enable = (val & 0b0000_0100) != 0;
                self.clock = match val & 0b0000_0011 {
                    0 => Clock::Div1024,
                    1 => Clock::Div16,
                    2 => Clock::Div64,
                    3 => Clock::Div256,
                    _ => panic!(),
                };
            }
            _ => {}
        }
    }
}

impl InterruptSource for Timer {
    fn pending(&self, _cycle: u64) -> InterruptMask {
        if self.irq_pending {
            Interrupt::Timer.as_mask()
        } else {
            InterruptMask::default()
        }
    }

    fn set_pending(&mut self, _cycle: u64, mask: InterruptMask) {
        self.irq_pending = mask.is_set(Interrupt::Timer);
    }

    fn next_pending(&self, _cycle: u64) -> u64 {
        u64::MAX
    }
}
