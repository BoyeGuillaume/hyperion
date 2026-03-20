use crate::api::{ApplicationInfo, VersionInfo};

/// CFFI-compatible version of `VersionInfo`, used for FFI boundaries
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HyVersionInfo {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl From<VersionInfo> for HyVersionInfo {
    fn from(info: VersionInfo) -> Self {
        Self {
            major: info.major,
            minor: info.minor,
            patch: info.patch,
        }
    }
}

impl From<HyVersionInfo> for VersionInfo {
    fn from(info: HyVersionInfo) -> Self {
        Self {
            major: info.major,
            minor: info.minor,
            patch: info.patch,
        }
    }
}

/// CFFI-compatible version of `ApplicationInfo`, used for FFI boundaries
#[repr(C)]
pub struct HyApplicationInfo {
    pub p_application_name: *const std::os::raw::c_char,
    pub application_version: HyVersionInfo,
    pub p_engine_name: *const std::os::raw::c_char,
    pub engine_version: HyVersionInfo,
}

impl HyApplicationInfo {
    pub unsafe fn to_application_info(&self) -> anyhow::Result<ApplicationInfo<'static>> {
        if self.p_application_name.is_null() {
            return Err(anyhow::anyhow!("Application name cannot be null"));
        }

        let application_name = unsafe { std::ffi::CStr::from_ptr(self.p_application_name) }
            .to_str()
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in application name: {}", e))?
            .to_owned();

        let engine_name = if self.p_engine_name.is_null() {
            None
        } else {
            Some(
                unsafe { std::ffi::CStr::from_ptr(self.p_engine_name) }
                    .to_str()
                    .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in engine name: {}", e))?
                    .to_owned(),
            )
        };

        Ok(ApplicationInfo {
            application_name: application_name.into(),
            application_version: self.application_version.into(),
            engine_name: engine_name.map(Into::into),
            engine_version: Some(self.engine_version.into()),
        })
    }
}

/// CFFI-compatible version of `InstanceCreateInfo`, used for FFI boundaries
#[repr(C)]
pub struct HyInstanceCreateInfo {
    pub p_application_info: *const HyApplicationInfo,
    pub enabled_plugin_count: u64,
    pub pp_enabled_plugins: *const *const std::os::raw::c_char,
    pub node_rank: u32,
    pub ext: *mut std::os::raw::c_void,
}
