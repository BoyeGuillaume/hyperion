use std::sync::Arc;

#[cfg(feature = "pyo3")]
use pyo3::{Borrowed, FromPyObject, PyAny, PyErr};
use strum::FromRepr;

use crate::{
    base::InstanceContext,
    utils::{error::HyResult, opaque::OpaqueList},
};

/// ABI-stable semantic version triple passed between frontends and the core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
#[repr(C)]
pub struct VersionInfo {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl From<VersionInfo> for semver::Version {
    fn from(val: VersionInfo) -> Self {
        semver::Version {
            major: val.major as u64,
            minor: val.minor as u64,
            patch: val.patch as u64,
            pre: semver::Prerelease::EMPTY,
            build: semver::BuildMetadata::EMPTY,
        }
    }
}

/// Describes the embedding application and target engine versions.
#[repr(C)]
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
pub struct ApplicationInfo {
    pub application_version: VersionInfo,
    pub application_name: String,
    pub engine_version: VersionInfo,
    pub engine_name: String,
}

/// Container used to request the creation of an [`InstanceContext`].
#[repr(C)]
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
pub struct InstanceCreateInfo {
    pub application_info: ApplicationInfo,
    pub enabled_extensions: Vec<String>,
    pub node_id: u32,
    pub ext: OpaqueList,
}

/// Enumeration of the different sources from which a module can be compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u32)]
pub enum ModuleSourceType {
    /// Contains assembly code
    Assembly,
}

#[cfg(feature = "pyo3")]
impl<'a, 'py> FromPyObject<'a, 'py> for ModuleSourceType {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, PyErr> {
        let level_int: u32 = obj.extract()?;
        if let Some(level) = ModuleSourceType::from_repr(level_int) {
            Ok(level)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid LogLevelEXT value: {}",
                level_int
            )))
        }
    }
}

/// Information about the source of a module.
#[repr(C)]
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
pub struct ModuleSourceInfo {
    pub source_type: ModuleSourceType,
    pub filename: Option<String>,
    pub data: String,
}

/// Structure containing information about how to compile a list of source files
#[repr(C)]
#[cfg_attr(feature = "pyo3", derive(FromPyObject))]
pub struct ModuleCompileInfo {
    pub sources: Vec<ModuleSourceInfo>,
}

/// Creates and initializes a new [`InstanceContext`] from the provided metadata.
pub fn create_instance(create_info: InstanceCreateInfo) -> HyResult<Arc<InstanceContext>> {
    InstanceContext::create(create_info)
}

pub use crate::compiler::{compile_sources, load_module};
