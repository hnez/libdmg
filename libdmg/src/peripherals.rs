mod audio;
mod interrupts;
mod joypad;
mod memory;
mod serial;
mod timer;
mod video;

pub use interrupts::{Interrupt, InterruptMask, InterruptSource};
pub use joypad::Button;
pub use memory::cartridge::Cartridge;

pub struct Peripherals {
    bootrom: memory::bootrom::BootRom,
    cartridge: Cartridge,
    video: video::Video,
    ram: memory::ram::Ram,
    joypad: joypad::Joypad,
    serial: serial::Serial,
    timer: timer::Timer,
    audio: audio::Audio,
    bootrom_mapped: bool,
    dma_reg: u8,
    ie_reg: u8,
}

impl Peripherals {
    pub(crate) fn new(bootrom: Vec<u8>, cartridge: Cartridge) -> Self {
        Self {
            bootrom: memory::bootrom::BootRom::new(bootrom),
            cartridge,
            video: video::Video::new(),
            ram: memory::ram::Ram::new(),
            joypad: joypad::Joypad::new(),
            serial: serial::Serial::new(),
            timer: timer::Timer::new(),
            audio: audio::Audio::new(),
            bootrom_mapped: true,
            dma_reg: 0,
            ie_reg: 0,
        }
    }

    pub(crate) fn buttons(&mut self, buttons: &[Button]) {
        self.joypad.buttons(buttons);
    }

    pub(crate) fn framebuffer(&mut self, cycle: u64) -> &[u8] {
        self.video.framebuffer(cycle)
    }

    pub(crate) fn write(&mut self, cycle: u64, addr: u16, val: u8) {
        match addr {
            0x0000..=0x00ff if self.bootrom_mapped => {}
            0x0000..=0x7fff => self.cartridge.write(addr, val),
            0x8000..=0x9fff => self.video.write(cycle, addr, val),
            0xa000..=0xbfff => self.cartridge.write(addr, val),
            0xc000..=0xfdff => self.ram.write(addr, val),
            0xfe00..=0xfe9f => self.video.write(cycle, addr, val),
            0xfea0..=0xfeff => {}
            0xff00 => self.joypad.write(val),
            0xff01..=0xff02 => self.serial.write(cycle, addr, val),
            0xff03 => {}
            0xff04..=0xff07 => self.timer.write(cycle, addr, val),
            0xff08..=0xff0e => {}
            0xff0f => self.set_pending(cycle, val.into()),
            0xff10..=0xff3f => self.audio.write(cycle, addr, val),
            0xff40..=0xff45 => self.video.write(cycle, addr, val),
            0xff46 => {
                // Special case OAM DMA transfers as they are quite anoying to handle
                // otherwise.
                // "But writing the whole OAM at once is cheating", you may say.
                // And I will say that I do not care and games don't either.

                for idx in 0..160 {
                    let virtual_cycle = cycle + idx * 4;
                    let src_addr = (val as u16) << 8 | (idx as u16);
                    let dst_addr = 0xfe00 | (idx as u16);

                    let val = self.read(virtual_cycle, src_addr);
                    self.write(virtual_cycle, dst_addr, val);
                }

                // Store the value, just in case somewone wants to read it
                self.dma_reg = val;
            }
            0xff47..=0xff4b => self.video.write(cycle, addr, val),
            0xff4c..=0xff4f => {}
            0xff50 => {
                // Special case the bootrom unmapping as well
                if val != 0 {
                    self.bootrom_mapped = false;
                }
            }
            0xff51..=0xff7f => {}
            0xff80..=0xfffe => self.ram.write(addr, val),
            0xffff => {
                self.ie_reg = val;
            }
        }
    }

    pub(crate) fn read(&self, cycle: u64, addr: u16) -> u8 {
        match addr {
            0x0000..=0x00ff if self.bootrom_mapped => self.bootrom.read(addr),
            0x0000..=0x7fff => self.cartridge.read(addr),
            0x8000..=0x9fff => self.video.read(cycle, addr),
            0xa000..=0xbfff => self.cartridge.read(addr),
            0xc000..=0xfdff => self.ram.read(addr),
            0xfe00..=0xfe9f => self.video.read(cycle, addr),
            0xfea0..=0xfeff => 0,
            0xff00 => self.joypad.read(),
            0xff01..=0xff02 => self.serial.read(cycle, addr),
            0xff03 => 0,
            0xff04..=0xff07 => self.timer.read(cycle, addr),
            0xff08..=0xff0e => 0,
            0xff0f => self.pending(cycle).into(),
            0xff10..=0xff3f => self.audio.read(cycle, addr),
            0xff40..=0xff45 => self.video.read(cycle, addr),
            0xff46 => self.dma_reg,
            0xff47..=0xff4b => self.video.read(cycle, addr),
            0xff4c..=0xff4f => 0,
            0xff50 => self.bootrom_mapped as u8,
            0xff51..=0xff7f => 0,
            0xff80..=0xfffe => self.ram.read(addr),
            0xffff => self.ie_reg,
        }
    }

    pub(crate) fn read_u16(&self, cycle: u64, addr: u16) -> u16 {
        let low = self.read(cycle, addr);
        let high = self.read(cycle, addr.wrapping_add(1));
        u16::from_le_bytes([low, high])
    }

    pub(crate) fn write_u16(&mut self, cycle: u64, addr: u16, val: u16) {
        let [low, high] = val.to_le_bytes();

        self.write(cycle, addr, low);
        self.write(cycle, addr.wrapping_add(1), high);
    }
}

impl InterruptSource for Peripherals {
    fn pending(&self, cycle: u64) -> InterruptMask {
        (self.video.pending(cycle) & (Interrupt::VBlank.as_mask() | Interrupt::Lcd.as_mask()))
            | (self.timer.pending(cycle) & Interrupt::Timer.as_mask())
            | (self.serial.pending(cycle) & Interrupt::Serial.as_mask())
            | (self.joypad.pending(cycle) & Interrupt::Joypad.as_mask())
    }

    fn set_pending(&mut self, cycle: u64, mask: InterruptMask) {
        self.video.set_pending(
            cycle,
            mask & (Interrupt::VBlank.as_mask() | Interrupt::Lcd.as_mask()),
        );
        self.timer
            .set_pending(cycle, mask & Interrupt::Timer.as_mask());
        self.serial
            .set_pending(cycle, mask & Interrupt::Serial.as_mask());
        self.joypad
            .set_pending(cycle, mask & Interrupt::Joypad.as_mask());
    }

    fn next_pending(&self, cycle: u64) -> u64 {
        let cycles = [
            self.video.next_pending(cycle),
            self.timer.next_pending(cycle),
            self.serial.next_pending(cycle),
            self.joypad.next_pending(cycle),
        ];

        cycles.into_iter().min().unwrap()
    }
}
