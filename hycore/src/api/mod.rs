use std::{borrow::Cow, path::PathBuf};

use bevy_tasks::tick_global_task_pools_on_main_thread;
use bitflags::bitflags;

use crate::{
    HyResult,
    ext::ExtList,
    hyerror,
    hyir::{compile_sources, load_compiled_module},
    instance::{Instance, core::ModuleHandle},
    schedule::Main,
};

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

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ModuleCompileInfoFlags: u32 {
        /// Whether to use ZSTD compression on the compiled module. This significantly reduces the size of the compiled module
        const ZSTD_COMPRESSION = 1 << 0;
    }
}

#[derive(Debug, Clone)]
pub struct ModuleCompileInfoSourceDescriptor {
    pub data: Option<String>,
    pub filename: Option<PathBuf>,
}

#[derive(Debug)]
pub struct ModuleCompileInfo {
    pub base_path: Option<PathBuf>,
    pub source_descriptors: Vec<ModuleCompileInfoSourceDescriptor>,
    pub flags: ModuleCompileInfoFlags,
    pub ext: ExtList,
}

pub fn hy_create_instance<'a>(create_info: InstanceCreateInfo<'a>) -> HyResult<Instance> {
    Instance::new(create_info)
}

pub fn hy_compile_module(
    instance: &Instance,
    compile_info: ModuleCompileInfo,
) -> HyResult<Vec<u8>> {
    compile_sources(instance, compile_info).inspect_err(|error| {
        hyerror!(instance; "{:?}", error);
    })
}

pub fn hy_version() -> semver::Version {
    let version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("Invalid version format in CARGO_PKG_VERSION");
    version
}

pub fn hy_load_compiled_module(instance: &mut Instance, data: &[u8]) -> HyResult<ModuleHandle> {
    load_compiled_module(instance, data).inspect_err(|error| {
        hyerror!(instance; "{:?}", error);
    })
}

pub fn hy_destroy_module(instance: &mut Instance, module_handle: ModuleHandle) -> HyResult<()> {
    instance.remove_module(module_handle).inspect_err(|error| {
        hyerror!(instance; "{:?}", error);
    })
}

pub fn hy_run(instance: &mut Instance) -> ! {
    loop {
        instance.world.run_schedule(Main);
        tick_global_task_pools_on_main_thread();
    }
}
