use pyo3::prelude::*;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn add(a: usize, b: usize) -> PyResult<usize> {
    Ok(libdmg::add(a, b))
}

/// A Python module implemented in Rust.
#[pymodule]
fn pydmg(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(add, m)?)?;
    Ok(())
}
