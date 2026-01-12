use std::sync::Arc;

#[cfg(feature = "pyo3")]
use pyo3::FromPyObject;

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

impl Into<semver::Version> for VersionInfo {
    fn into(self) -> semver::Version {
        semver::Version {
            major: self.major as u64,
            minor: self.minor as u64,
            patch: self.patch as u64,
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
    pub ext: OpaqueList,
}

/// Creates and initializes a new [`InstanceContext`] from the provided metadata.
pub unsafe fn create_instance(create_info: InstanceCreateInfo) -> HyResult<Arc<InstanceContext>> {
    unsafe { InstanceContext::create(create_info) }
}
