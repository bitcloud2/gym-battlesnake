mod gamewrapper;

pub use gamewrapper::GameWrapper;

use pyo3::prelude::{pymodule, PyModule, PyResult, Python};

// The name of the module must be the same as the rust package name
#[pymodule]
fn rust(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<GameWrapper>()?;
    Ok(())
}
