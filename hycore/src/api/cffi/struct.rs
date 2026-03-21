use anyhow::Context;
use strum::FromRepr;

use crate::{
    api::{ApplicationInfo, VersionInfo},
    ext::ExtList,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, FromRepr)]
#[repr(u32)]
pub enum HyStructureType {
    ApplicationInfo = 1,
    InstanceCreateInfo,
    LoggerPluginCreateInfo,
}

impl HyStructureType {
    #[inline]
    pub fn from_u32(value: u32) -> Option<Self> {
        Self::from_repr(value)
    }

    #[inline]
    pub fn to_u32(self) -> u32 {
        self as u32
    }

    #[inline]
    pub fn is_valid(value: u32) -> bool {
        Self::from_u32(value).is_some()
    }
}

/// CFFI-compatible version of `VersionInfo`, used for FFI boundaries
#[derive(Clone, Copy)]
#[repr(C)]
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
    pub s_type: HyStructureType,
    pub p_application_name: *const std::os::raw::c_char,
    pub application_version: HyVersionInfo,
    pub p_engine_name: *const std::os::raw::c_char,
    pub engine_version: HyVersionInfo,
}

impl HyApplicationInfo {
    pub unsafe fn to_application_info(&self) -> anyhow::Result<ApplicationInfo<'static>> {
        if self.s_type != HyStructureType::ApplicationInfo {
            return Err(anyhow::anyhow!(
                "Invalid structure type: expected ApplicationInfo, got {:?}",
                self.s_type
            ));
        }

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
    pub s_type: HyStructureType,
    pub p_application_info: *const HyApplicationInfo,
    pub enabled_plugin_count: u64,
    pub pp_enabled_plugins: *const *const std::os::raw::c_char,
    pub node_rank: u64,
    pub p_next: *mut std::os::raw::c_void,
}

impl HyInstanceCreateInfo {
    pub unsafe fn to_instance_create_info(
        &self,
    ) -> anyhow::Result<crate::api::InstanceCreateInfo<'static>> {
        if self.s_type != HyStructureType::InstanceCreateInfo {
            return Err(anyhow::anyhow!(
                "Invalid structure type: expected InstanceCreateInfo, got {:?}",
                self.s_type
            ));
        }

        if self.p_application_info.is_null() {
            return Err(anyhow::anyhow!(
                "Application info pointer cannot be null in InstanceCreateInfo"
            ));
        }

        let application_info = unsafe { (*self.p_application_info).to_application_info()? };
        let enabled_plugins = if self.enabled_plugin_count > 0 {
            if self.pp_enabled_plugins.is_null() {
                return Err(anyhow::anyhow!(
                    "Enabled plugin count is {}, but ppEnabledPlugins is null",
                    self.enabled_plugin_count
                ));
            }

            let mut plugins = Vec::with_capacity(self.enabled_plugin_count as usize);
            for i in 0..self.enabled_plugin_count {
                let plugin_ptr = unsafe { *self.pp_enabled_plugins.add(i as usize) };
                if plugin_ptr.is_null() {
                    return Err(anyhow::anyhow!("Plugin name at index {} is null", i));
                }
                let plugin_name = unsafe { std::ffi::CStr::from_ptr(plugin_ptr) }
                    .to_str()
                    .map_err(|e| {
                        anyhow::anyhow!("Invalid UTF-8 in plugin name at index {}: {}", i, e)
                    })?
                    .to_owned();
                plugins.push(plugin_name.into());
            }
            plugins
        } else {
            Vec::new()
        };

        // Build the ext list
        Ok(crate::api::InstanceCreateInfo {
            application_info,
            enabled_plugins,
            node_rank: self.node_rank,
            // ext: self.ext,
            ext: unsafe {
                ExtList::from_cffi(self.p_next)
                    .with_context(|| format!("Failed to build ExtList from p_next"))?
            },
        })
    }
}
