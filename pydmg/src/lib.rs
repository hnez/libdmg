use pyo3::prelude::*;

#[pyclass]
struct Cartridge {}

#[pymethods]
impl Cartridge {
    #[new]
    fn new(rom: &[u8], sram: Option<&[u8]>) -> Self {
        println!(
            "rom: {} bytes, sram: {} bytes",
            rom.len(),
            sram.map(|s| s.len()).unwrap_or(0)
        );

        Self {}
    }
}

#[pymodule]
fn pydmg(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Cartridge>()?;
    Ok(())
}
