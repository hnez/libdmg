use std::sync::Arc;

#[derive(Clone)]
pub struct BootRom {
    rom: Arc<[u8]>,
}

impl BootRom {
    pub(crate) fn new(rom: Vec<u8>) -> Self {
        assert!(rom.len() == 256);

        let rom = rom.into_boxed_slice().into();

        Self { rom }
    }

    pub(crate) fn read(&self, addr: u16) -> u8 {
        self.rom[addr as usize]
    }
}
