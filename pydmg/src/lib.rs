use numpy::PyArray2;
use pyo3::{exceptions::PyValueError, prelude::*};

#[pyclass]
#[derive(Clone)]
struct Cartridge(libdmg::Cartridge);

#[pymethods]
impl Cartridge {
    #[new]
    fn new(rom: Vec<u8>, sram: Option<Vec<u8>>) -> Self {
        Self(libdmg::Cartridge::new(rom, sram))
    }
}

#[pyclass]
struct Dmg(libdmg::Dmg);

#[pymethods]
impl Dmg {
    #[new]
    fn new(bootrom: Vec<u8>, cartridge: Cartridge) -> Self {
        Self(libdmg::Dmg::new(bootrom, cartridge.0))
    }

    fn run_frame(&mut self, framebuffer: &PyArray2<u8>) -> PyResult<()> {
        if framebuffer.shape() != &[160, 144] {
            return Err(PyValueError::new_err("framebuffer must have shape 160x144"))?;
        }

        let frame_src = self.0.run_frame(&[]);

        let mut framebuffer = framebuffer.readwrite();
        let frame_dst = framebuffer.as_slice_mut()?;

        frame_dst
            .iter_mut()
            .zip(frame_src)
            .for_each(|(dst, src)| *dst = *src);

        Ok(())
    }
}

#[pymodule]
fn pydmg(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Cartridge>()?;
    m.add_class::<Dmg>()?;

    Ok(())
}
