use crate::peripherals::Peripherals;

pub struct PcReader<'a> {
    cycle: u64,
    pc: &'a mut u16,
    peripherals: &'a Peripherals,
}

impl<'a> PcReader<'a> {
    pub(super) fn new(cycle: u64, pc: &'a mut u16, peripherals: &'a Peripherals) -> Self {
        Self {
            cycle,
            pc,
            peripherals,
        }
    }

    pub fn read_u8(&mut self) -> u8 {
        let res = self.peripherals.read(self.cycle, *self.pc);
        *self.pc += 1;
        res
    }

    pub fn read_u16(&mut self) -> u16 {
        let res = self.peripherals.read_u16(self.cycle, *self.pc);
        *self.pc += 2;
        res
    }
}
