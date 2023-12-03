pub struct Audio;

impl Audio {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn read(&self, _cycle: u64, _addr: u16) -> u8 {
        0
    }

    pub(crate) fn write(&mut self, _cycle: u64, _addr: u16, _val: u8) {}
}
