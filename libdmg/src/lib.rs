mod cpu;
mod peripherals;

use cpu::Cpu;
use peripherals::Peripherals;
pub use peripherals::{Button, Cartridge};

pub struct Dmg {
    cpu: Cpu,
    peripherals: Peripherals,
}

impl Dmg {
    pub fn new(bootrom: Vec<u8>, cartridge: Cartridge) -> Self {
        Self {
            cpu: Cpu::new(),
            peripherals: Peripherals::new(bootrom, cartridge),
        }
    }

    pub fn run_frame(&mut self, buttons: &[Button]) -> &[u8] {
        self.peripherals.buttons(buttons);

        // TODO: make sure we run until vblank
        self.cpu.run(&mut self.peripherals, 70224);

        self.peripherals.framebuffer(self.cpu.cycle())
    }
}
