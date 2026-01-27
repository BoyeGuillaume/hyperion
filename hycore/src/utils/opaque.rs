//! Utilities that allow extensions to pass opaque configuration objects across
//! the FFI boundary (notably from Python) into Rust plugins.
use std::fmt::Debug;

use downcast_rs::{DowncastSync, impl_downcast};
#[cfg(feature = "pyo3")]
use pyo3::{FromPyObject, PyAny, PyResult};

/// Marker trait implemented by per-extension configuration structs that need to
/// cross API boundaries without the host knowing their concrete type upfront.
pub trait OpaqueObject: DowncastSync + Debug {}
impl_downcast!(sync OpaqueObject);

#[cfg(feature = "pyo3")]
/// Python-side factory that converts a `PyAny` into an [`OpaqueObject`]. Each
/// registered loader is keyed by the fully-qualified Python class name so
/// plugins can bring their own dataclasses.
pub type PyOpaqueObjectLoader =
    fn(pyo3::Borrowed<'_, '_, PyAny>) -> PyResult<Box<dyn OpaqueObject>>;
#[cfg(not(feature = "pyo3"))]
pub type PyOpaqueObjectLoader = fn() -> ();

/// A registry of Python-side loaders for opaque configuration objects.
#[cfg(feature = "pyo3")]
pub struct PyOpaqueObjectLoadersRegistry {
    pub name: &'static str,
    pub callback: fn(pyo3::Borrowed<'_, '_, PyAny>) -> PyResult<Box<dyn OpaqueObject>>,
}
#[cfg(feature = "pyo3")]
inventory::collect!(PyOpaqueObjectLoadersRegistry);

#[macro_export]
#[cfg(not(feature = "pyo3"))]
macro_rules! define_py_opaque_object_loaders {
    (
        $name:literal,
        $obj_type:ty
        $(,)?
    ) => {};
}
#[macro_export]
#[cfg(feature = "pyo3")]
macro_rules! define_py_opaque_object_loaders {
    (
        $name:expr, $obj_type:ty
        $(,)?
    ) => {
        inventory::submit! {
            $crate::utils::opaque::PyOpaqueObjectLoadersRegistry {
                name: $name,
                callback: |obj| -> pyo3::PyResult<Box<dyn OpaqueObject>> {
                    let extracted: $obj_type = obj.extract()?;
                    Ok(Box::new(extracted))
                },
            }
        }
    };
}

/// Bag of dynamically typed configuration entries supplied when an instance is
/// created. Plugins inspect and extract the structs relevant to them.
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
#[derive(Debug, Default)]
pub struct OpaqueList(pub Vec<Box<dyn OpaqueObject>>);

impl OpaqueList {
    /// Retrieve an extension object by type and remove it from the list.
    pub fn take_ext<T: OpaqueObject + 'static>(&mut self) -> Option<Box<T>> {
        if let Some(pos) = self.0.iter().position(|ext| ext.as_ref().is::<T>()) {
            let ext = self.0.remove(pos);
            Some(ext.downcast::<T>().ok().unwrap())
        } else {
            None
        }
    }
}

#[cfg(feature = "pyo3")]
impl<'a, 'py> FromPyObject<'a, 'py> for Box<dyn OpaqueObject> {
    type Error = pyo3::PyErr;

    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        use pyo3::types::{PyAnyMethods, PyTypeMethods};

        let type_name: String = obj.get_type().fully_qualified_name()?.to_string();

        // Find if there is a registered loader for this type.
        for registry in inventory::iter::<PyOpaqueObjectLoadersRegistry> {
            if registry.name == type_name {
                return (registry.callback)(obj);
            }
        }

        println!(
            "No loader registered for type '{}'. Possible types are: {:?}",
            type_name,
            inventory::iter::<PyOpaqueObjectLoadersRegistry>
                .into_iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
        );

        // If we reach here, no loader was found.
        Err(pyo3::exceptions::PyTypeError::new_err(format!(
            "No loader registered for type '{}'. Possible types are: {:?}",
            type_name,
            inventory::iter::<PyOpaqueObjectLoadersRegistry>
                .into_iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
        )))
    }
}
