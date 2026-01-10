use downcast_rs::{DowncastSync, impl_downcast};
#[cfg(feature = "pyo3")]
use phf::phf_map;
#[cfg(feature = "pyo3")]
use pyo3::{FromPyObject, PyAny, PyResult};

/// Extendable configuration trait for dynamic extension of configuration structures.
pub trait OpaqueObject: DowncastSync {}
impl_downcast!(sync OpaqueObject);

/// A map of each py-object loader by type name.
#[cfg(feature = "pyo3")]
static PY_OBJECT_LOADERS: phf::Map<
    &'static str,
    fn(pyo3::Borrowed<'_, '_, PyAny>) -> PyResult<Box<dyn OpaqueObject>>,
> = phf_map! {
//     "my_module.MyType" => Box::new(|obj| {
};

/// A list of extendable configuration entries.
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
pub struct ExtList(pub Vec<Box<dyn OpaqueObject>>);

#[cfg(feature = "pyo3")]
impl<'a, 'py> FromPyObject<'a, 'py> for Box<dyn OpaqueObject> {
    type Error = pyo3::PyErr;

    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        use pyo3::types::{PyAnyMethods, PyTypeMethods};

        let type_name: String = obj.get_type().fully_qualified_name()?.to_string();

        if let Some(loader) = PY_OBJECT_LOADERS.get(&type_name) {
            loader(obj)
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "No loader registered for type '{}'",
                type_name
            )))
        }
    }
}
