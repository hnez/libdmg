#[derive(Clone)]
pub struct Ram {
    work_ram: [u8; 8192],
    high_ram: [u8; 127],
}

impl Ram {
    pub fn new() -> Self {
        Self {
            work_ram: [0u8; 8192],
            high_ram: [0u8; 127],
        }
    }

    pub(crate) fn read(&self, addr: u16) -> u8 {
        match addr {
            0xc000..=0xdfff => {
                let offset = (addr as usize) - 0xc000;
                self.work_ram[offset]
            }
            0xe000..=0xfdff => {
                let offset = (addr as usize) - 0xe000;
                self.work_ram[offset]
            }
            0xff80..=0xfffe => {
                let offset = (addr as usize) - 0xff80;
                self.high_ram[offset]
            }
            _ => panic!("Address {addr} is not in RAM space"),
        }
    }

    pub(crate) fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xc000..=0xdfff => {
                let offset = (addr as usize) - 0xc000;
                self.work_ram[offset] = val;
            }
            0xe000..=0xfdff => {
                let offset = (addr as usize) - 0xe000;
                self.work_ram[offset] = val;
            }
            0xff80..=0xfffe => {
                let offset = (addr as usize) - 0xff80;
                self.high_ram[offset] = val;
            }
            _ => panic!("Address {addr} is not in RAM space"),
        }
    }
}
