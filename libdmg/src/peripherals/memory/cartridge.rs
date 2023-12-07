use std::sync::Arc;

use log::{error, info};

#[derive(Clone)]
pub struct Cartridge {
    rom: Arc<[u8]>,
    ram: Vec<u8>,
    rom_bank: u8,
    ram_bank: u8,
    ram_write_enable: bool,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>, ram: Option<Vec<u8>>) -> Self {
        let rom = rom.into_boxed_slice().into();
        let ram = ram.unwrap_or_else(|| vec![0u8; 4 * 8192]);

        let rom_bank = 1;
        let ram_bank = 0;
        let ram_write_enable = false;

        Self {
            rom,
            ram,
            rom_bank,
            ram_bank,
            ram_write_enable,
        }
    }

    pub(crate) fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3fff => {
                let offset = addr as usize;
                self.rom[offset]
            }
            0x4000..=0x7fff => {
                let offset = (addr as usize) - 0x4000;
                let bank_base = (self.rom_bank as usize) * 16384;
                self.rom[bank_base + offset]
            }
            0xa000..=0xbfff => {
                let offset = (addr as usize) - 0xa000;
                let bank_base = (self.ram_bank as usize) * 8192;
                self.ram[bank_base + offset]
            }
            _ => panic!("Address {addr} is not in cartidge space"),
        }
    }

    pub(crate) fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1fff => {
                match val {
                    0x0a => self.ram_write_enable = true,
                    0x00 => {
                        // TODO: notify user to save the cartridge ram now
                        self.ram_write_enable = false;
                    }
                    _ => info!(
                        "Tried to write 0x{val:02x} to 0x{addr:04x} which is not a valid value"
                    ),
                }
            }
            0x2000..=0x3fff => {
                let bank = val & 0x7f;
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            0x4000..=0x5fff => {
                if val < 4 {
                    self.ram_bank = val;
                } else {
                    error!("Enabling the RTC registers is not implemented");
                }
            }
            0x6000..=0x7fff => {
                error!("Latching RTC data is not implemented");
            }
            0xa000..=0xbfff => {
                if self.ram_write_enable {
                    let offset = (addr as usize) - 0xa000;
                    let bank_base = (self.ram_bank as usize) * 8192;
                    self.ram[bank_base + offset] = val;
                } else {
                    info!("Tried to write 0x{val:02x} to 0x{addr:04x} while ram write disabled");
                }
            }
            _ => panic!("Address {addr} is not in cartidge space"),
        }
    }
}
