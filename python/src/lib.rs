use std::sync::Arc;

use hycore::base::{InstanceContext, api};
use pyo3::{intern, prelude::*};

#[pyclass]
pub struct Instance(Arc<InstanceContext>);

fn into_version<'py>(object: &Bound<'py, PyAny>) -> PyResult<api::VersionInfo> {
    let major: u16 = object.getattr(intern!(object.py(), "major"))?.extract()?;
    let minor: u16 = object.getattr(intern!(object.py(), "minor"))?.extract()?;
    let patch: u16 = object.getattr(intern!(object.py(), "patch"))?.extract()?;
    Ok(api::VersionInfo {
        major,
        minor,
        patch,
    })
}

#[pyfunction]
fn _hy_create_instance<'py>(instance_create_info: &Bound<'py, PyAny>) -> PyResult<Instance> {
    // Create the instance object from the provided create info
    let application_info_obj =
        instance_create_info.getattr(intern!(instance_create_info.py(), "application_info"))?;

    let application_version = into_version(
        &application_info_obj.getattr(intern!(instance_create_info.py(), "application_version"))?,
    )?;
    let application_name_obj =
        application_info_obj.getattr(intern!(instance_create_info.py(), "application_name"))?;
    let application_name: &str = application_name_obj.extract()?;

    let engine_version = into_version(
        &application_info_obj.getattr(intern!(instance_create_info.py(), "engine_version"))?,
    )?;
    let engine_name_obj =
        application_info_obj.getattr(intern!(instance_create_info.py(), "engine_name"))?;
    let engine_name: &str = engine_name_obj.extract()?;

    let application_info = api::ApplicationInfo {
        application_version,
        application_name,
        engine_version,
        engine_name,
    };

    // Enabled extensions
    let enabled_extensions_obj: Vec<String> = instance_create_info
        .getattr(intern!(instance_create_info.py(), "enabled_extensions"))?
        .extract()?;

    let enabled_extensions: Vec<&str> = enabled_extensions_obj.iter().map(|s| s.as_str()).collect();
    let create_info = api::InstanceCreateInfo {
        application_info: &application_info,
        enabled_extensions: &enabled_extensions,
    };

    let instance_context = unsafe { api::create_instance(&create_info) }.map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to create instance: {}",
            e
        ))
    })?;

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
