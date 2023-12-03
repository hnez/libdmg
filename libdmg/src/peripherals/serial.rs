use log::warn;

use super::{Interrupt, InterruptMask, InterruptSource};

#[derive(Clone, Copy)]
enum Clock {
    External,
    Internal,
}

pub struct Serial {
    data: u8,
    transfer_enable: bool,
    clock_select: Clock,
    irq_pending: bool,
}

impl Serial {
    pub(crate) fn new() -> Self {
        Self {
            data: 0,
            transfer_enable: false,
            clock_select: Clock::External,
            irq_pending: false,
        }
    }

    pub(crate) fn read(&self, _cycle: u64, addr: u16) -> u8 {
        match addr {
            0xff01 => {
                warn!("read data from serial data register which never makes progress");
                self.data
            }
            0xff02 => {
                warn!("read data from serial control register which never makes progress");
                (self.transfer_enable as u8) << 7 | (self.clock_select as u8)
            }
            _ => 0,
        }
    }

    pub(crate) fn write(&mut self, _cycle: u64, addr: u16, val: u8) {
        match addr {
            0xff01 => {
                warn!("wrote data to serial data register which never makes progress");
                self.data = val;
            }
            0xff02 => {
                warn!("wrote data to serial control register which never makes progress");
                self.transfer_enable = (val & 0b1000_0000) != 0;
                self.clock_select = match val & 0b0000_0001 {
                    0 => Clock::External,
                    1 => Clock::Internal,
                    _ => panic!(),
                };
            }
            _ => {}
        }
    }
}

impl InterruptSource for Serial {
    fn pending(&self, _cycle: u64) -> InterruptMask {
        if self.irq_pending {
            Interrupt::Serial.as_mask()
        } else {
            InterruptMask::default()
        }
    }

    fn set_pending(&mut self, _cycle: u64, mask: InterruptMask) {
        self.irq_pending = mask.is_set(Interrupt::Serial);
    }

    fn next_pending(&self, _cycle: u64) -> u64 {
        u64::MAX
    }
}
