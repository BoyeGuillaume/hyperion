use std::sync::Arc;

use crate::{base::InstanceContext, utils::error::HyResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationInfo<'a> {
    pub application_version: VersionInfo,
    pub application_name: &'a str,
    pub engine_version: VersionInfo,
    pub engine_name: &'a str,
}

#[repr(C)]
pub struct InstanceCreateInfo<'a> {
    pub application_info: &'a ApplicationInfo<'a>,
    pub enabled_extensions: &'a [&'a str],
}

pub unsafe fn create_instance(create_info: &InstanceCreateInfo) -> HyResult<Arc<InstanceContext>> {
    unsafe { InstanceContext::create(create_info) }
}
