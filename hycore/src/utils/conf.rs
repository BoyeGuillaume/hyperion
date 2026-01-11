#[cfg(feature = "pyo3")]
use std::collections::BTreeMap;

use downcast_rs::{DowncastSync, impl_downcast};
#[cfg(feature = "pyo3")]
use parking_lot::RwLock;
#[cfg(feature = "pyo3")]
#[cfg(feature = "pyo3")]
use pyo3::{FromPyObject, PyAny, PyResult};

/// Extendable configuration trait for dynamic extension of configuration structures.
pub trait OpaqueObject: DowncastSync {}
impl_downcast!(sync OpaqueObject);

#[cfg(feature = "pyo3")]
pub type PyOpaqueObjectLoader =
    fn(pyo3::Borrowed<'_, '_, PyAny>) -> PyResult<Box<dyn OpaqueObject>>;
#[cfg(not(feature = "pyo3"))]
pub type PyOpaqueObjectLoader = fn() -> ();

/// A map of each py-object loader by type name.
#[cfg(feature = "pyo3")]
pub static PY_OPAQUE_OBJECT_LOADERS: RwLock<BTreeMap<String, PyOpaqueObjectLoader>> =
    RwLock::new(BTreeMap::new());

/// A list of extendable configuration entries.
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
            println!("Found loader for type '{}'", type_name);
            loader(obj).inspect_err(|e| {
                println!("Failed: {}", e);
            })
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "No loader registered for type '{}'. Possible types are: {:?}",
                type_name,
                PY_OPAQUE_OBJECT_LOADERS.read().keys().collect::<Vec<_>>()
            )))
        }
    }
}
