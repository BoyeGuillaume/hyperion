use std::sync::Arc;

use hycore::base::{InstanceContext, api, ext::preload_plugins};
use pyo3::{intern, prelude::*};

#[pyclass]
#[allow(dead_code)]
pub struct Instance(Arc<InstanceContext>);

#[pyfunction]
fn _hy_create_instance<'py>(instance_create_info: &Bound<'py, PyAny>) -> PyResult<Instance> {
    let py = instance_create_info.py();

    // Start by preloading all plugins requested in the create info
    let ext_names: Vec<String> = instance_create_info
        .getattr(intern!(py, "enabled_extensions"))?
        .extract()?;
    let guard = unsafe { preload_plugins(ext_names) }.map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to preload plugins: {}",
            e
        ))
    })?;

    // Create the instance object from the provided create info
    let create_info: api::InstanceCreateInfo = instance_create_info.extract().map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("Invalid InstanceCreateInfo: {}", e))
    })?;

    let instance_context = unsafe { api::create_instance(create_info) }.map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to create instance: {}",
            e
        ))
    })?;

    drop(guard); // Release the preload guard
    Ok(Instance(instance_context))
}

#[pyfunction]
/// Computes the factorial of a number.
fn factorial(n: u64) -> PyResult<u64> {
    let mut res = 1u64;
    for i in 2..=n {
        res = res.checked_mul(i).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyOverflowError, _>("integer overflow")
        })?;
    }
    Ok(res)
}

#[pyfunction]
/// Computes the fibonacci of a number.
fn fibonacci(n: u64) -> PyResult<u64> {
    let mut a = 0u64;
    let mut b = 1u64;
    for _ in 0..n {
        let temp = a;
        a = b;
        b = temp.checked_add(b).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyOverflowError, _>("integer overflow")
        })?;
    }
    Ok(a)
}

#[pymodule]
#[pyo3(name = "_sys")]
#[pyo3(submodule)]
fn hypi_sys(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Instance>()?;

    m.add_function(wrap_pyfunction!(_hy_create_instance, m)?)?;

    m.add_function(wrap_pyfunction!(factorial, m)?)?;
    m.add_function(wrap_pyfunction!(fibonacci, m)?)?;
    Ok(())
}
