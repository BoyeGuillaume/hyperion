use std::borrow::Cow;

use crate::{HyResult, ext::ExtList, instance::Instance};

#[cfg(feature = "cffi")]
pub mod cffi;
pub mod constants;
pub mod function;
#[cfg(feature = "pyo3")]
pub mod pyo3;

/// Version information about the library, used for debugging and distinguishing between usecases
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionInfo {
    /// Major version, incremented for breaking changes
    pub major: u16,
    /// Minor version, incremented for new features and non-breaking changes
    pub minor: u16,
    /// Patch version, incremented for bug fixes and other minor changes
    pub patch: u16,
}

impl std::fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for VersionInfo {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow::anyhow!(
                "Invalid version string: \"{}\". Expected format: major.minor.patch",
                s
            ));
        }

        let major = parts[0]
            .parse::<u16>()
            .map_err(|e| anyhow::anyhow!("Invalid major version: {}", e))?;
        let minor = parts[1]
            .parse::<u16>()
            .map_err(|e| anyhow::anyhow!("Invalid minor version: {}", e))?;
        let patch = parts[2]
            .parse::<u16>()
            .map_err(|e| anyhow::anyhow!("Invalid patch version: {}", e))?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ApplicationInfo<'a> {
    /// Name of the application using the library
    pub application_name: Cow<'a, str>,

    /// Version of the application using the library
    pub application_version: VersionInfo,

    /// Name of the engine using the library, if any
    pub engine_name: Option<Cow<'a, str>>,

    /// Version of the engine using the library, if any.
    pub engine_version: Option<VersionInfo>,
}

#[derive(Debug)]
pub struct InstanceCreateInfo<'a> {
    /// Information about the library to distinguish for distinguishing between usecases and for debugging purposes.
    pub application_info: ApplicationInfo<'a>,

    /// A list of enabled plugins. You can check the [`constants`] module for the available (default) plugins. Additional
    /// library and module may add their own plugins
    pub enabled_plugins: Vec<String>,

    /// Only the 6 least significant bytes of the node rank are used, everything else is ignored and should be set to 0
    pub node_rank: u64,

    /// Extension point
    pub ext: ExtList,
}

pub fn hy_create_instance<'a>(create_info: InstanceCreateInfo<'a>) -> HyResult<Instance> {
    Instance::new(create_info)
}
