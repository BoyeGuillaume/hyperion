use std::sync::{Arc, Weak};

use hycore::base::{InstanceContext, ModuleKey, api};
use pyo3::{prelude::*, types::PyBytes};

/// Opaque handle to a running Hyperion instance exposed to Python callers.
#[pyclass]
#[allow(dead_code)]
pub struct Instance(Arc<InstanceContext>);

/// Opaque handle to a hyperion module
#[pyclass]
#[allow(dead_code)]
pub struct Module(ModuleKey, Weak<InstanceContext>);

impl Module {
    pub fn assert_instance(&self, instance: &Instance) -> PyResult<()> {
        if let Some(inst) = self.1.upgrade() {
            if Arc::ptr_eq(&inst, &instance.0) {
                Ok(())
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Module does not belong to the given instance",
                ))
            }
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Module's instance has been dropped",
            ))
        }
    }
}

/// Creates a new Hyperion instance from the validated Python dataclasses.
#[pyfunction]
fn _hy_create_instance<'py>(instance_create_info: &Bound<'py, PyAny>) -> PyResult<Instance> {
    // Create the instance object from the provided create info
    let create_info: api::InstanceCreateInfo = instance_create_info.extract().map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("Invalid InstanceCreateInfo: {}", e))
    })?;

    let instance_context = api::create_instance(create_info).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to create instance: {}",
            e
        ))
    })?;

    Ok(Instance(instance_context))
}

/// Compiles a list of source modules into a compiled module.
#[pyfunction]
fn _hy_compile_module<'py>(
    instance: &Instance,
    compile_info: &Bound<'py, PyAny>,
) -> PyResult<Py<PyAny>> {
    let py = compile_info.py();

    let compile_info: api::ModuleCompileInfo = compile_info.extract().map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("Invalid ModuleCompileInfo: {}", e))
    })?;

    let compiled_module = api::compile_sources(&instance.0, compile_info).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to compile module: {}",
            e
        ))
    })?;

    Ok(PyBytes::new(py, &compiled_module).into())
}

/// Loads a compiled module into the given instance.
#[pyfunction]
fn _hy_load_module<'py>(
    instance: &Instance,
    module_data: &Bound<'py, PyBytes>,
) -> PyResult<Module> {
    let data = module_data.as_bytes();
    let module_key = api::load_module(&instance.0, data).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "Failed to load compiled module: {}",
            e
        ))
    })?;
    Ok(Module(module_key, Arc::downgrade(&instance.0)))
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
    m.add_class::<Module>()?;

    m.add_function(wrap_pyfunction!(_hy_create_instance, m)?)?;
    m.add_function(wrap_pyfunction!(_hy_compile_module, m)?)?;
    m.add_function(wrap_pyfunction!(_hy_load_module, m)?)?;

    m.add_function(wrap_pyfunction!(factorial, m)?)?;
    m.add_function(wrap_pyfunction!(fibonacci, m)?)?;
    Ok(())
}
