use std::sync::Arc;

use hycore::base::{InstanceContext, api};
use pyo3::prelude::*;

/// Opaque handle to a running Hyperion instance exposed to Python callers.
#[pyclass]
#[allow(dead_code)]
pub struct Instance(Arc<InstanceContext>);

/// Creates a new Hyperion instance from the validated Python dataclasses.
#[pyfunction]
fn _hy_create_instance<'py>(instance_create_info: &Bound<'py, PyAny>) -> PyResult<Instance> {
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

    Ok(Instance(instance_context))
}

/// Computes the factorial of a number.
#[pyfunction]
fn factorial(n: u64) -> PyResult<u64> {
    let mut res = 1u64;
    for i in 2..=n {
        res = res.checked_mul(i).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyOverflowError, _>("integer overflow")
        })?;
    }
    Ok(res)
}

/// Computes the fibonacci of a number.
#[pyfunction]
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

/// Module initializer that wires the Rust functions into `hypi._sys`.
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
