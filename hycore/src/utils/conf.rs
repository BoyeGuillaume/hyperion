//! Utilities that allow extensions to pass opaque configuration objects across
//! the FFI boundary (notably from Python) into Rust plugins.

#[cfg(feature = "pyo3")]
use std::collections::BTreeMap;

use downcast_rs::{DowncastSync, impl_downcast};
#[cfg(feature = "pyo3")]
use parking_lot::RwLock;
#[cfg(feature = "pyo3")]
#[cfg(feature = "pyo3")]
use pyo3::{FromPyObject, PyAny, PyResult};

/// Marker trait implemented by per-extension configuration structs that need to
/// cross API boundaries without the host knowing their concrete type upfront.
pub trait OpaqueObject: DowncastSync {}
impl_downcast!(sync OpaqueObject);

#[cfg(feature = "pyo3")]
/// Python-side factory that converts a `PyAny` into an [`OpaqueObject`]. Each
/// registered loader is keyed by the fully-qualified Python class name so
/// plugins can bring their own dataclasses.
pub type PyOpaqueObjectLoader =
    fn(pyo3::Borrowed<'_, '_, PyAny>) -> PyResult<Box<dyn OpaqueObject>>;
#[cfg(not(feature = "pyo3"))]
pub type PyOpaqueObjectLoader = fn() -> ();

/// A map of each py-object loader by type name.
#[cfg(feature = "pyo3")]
pub static PY_OPAQUE_OBJECT_LOADERS: RwLock<BTreeMap<String, PyOpaqueObjectLoader>> =
    RwLock::new(BTreeMap::new());

/// Bag of dynamically typed configuration entries supplied when an instance is
/// created. Plugins inspect and extract the structs relevant to them.
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
pub struct ExtList(pub Vec<Box<dyn OpaqueObject>>);

impl ExtList {
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

        if let Some(loader) = PY_OPAQUE_OBJECT_LOADERS.read().get(&type_name) {
            loader(obj)
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "No loader registered for type '{}'. Possible types are: {:?}",
                type_name,
                PY_OPAQUE_OBJECT_LOADERS.read().keys().collect::<Vec<_>>()
            )))
        }
    }
}
