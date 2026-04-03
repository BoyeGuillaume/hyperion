use std::path::PathBuf;

use anyhow::Context;
use strum::FromRepr;

use crate::{
    HyResult,
    api::{
        ApplicationInfo, ModuleCompileInfoFlags, ModuleCompileInfoSourceDescriptor, VersionInfo,
    },
    ext::ExtList,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, FromRepr)]
#[repr(u32)]
pub enum HyStructureType {
    ApplicationInfo = 1,
    InstanceCreateInfo = 2,
    ModuleCompileInfo = 3,
    LoggerPluginCreateInfo = 0x8000,
    StartRemoteServerInfo = 0x8001,
    TlsServerCertificateInfo = 0x8002,
    TlsClientAuthentificationInfo = 0x8003,
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

#[repr(u32)]
pub enum HyModuleCompileInfoFlagBits {
    EnableZstdCompression = 1 << 0,
}

pub type HyModuleCompileInfoFlags = u32;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct HyModuleCompileInfoSourceDescriptor {
    pub p_data: *const std::os::raw::c_char,
    pub data_size: u32,
    pub p_filename: *const std::os::raw::c_char,
    pub filename_size: u32,
}

impl HyModuleCompileInfoSourceDescriptor {
    fn from_cffi(self) -> HyResult<ModuleCompileInfoSourceDescriptor> {
        let data = {
            if self.p_data.is_null() || self.data_size == 0 {
                None
            } else {
                let data_slice = unsafe {
                    std::slice::from_raw_parts(self.p_data as *const u8, self.data_size as usize)
                };
                Some(str::from_utf8(data_slice)?.to_string())
            }
        };

        let filename = {
            if self.p_filename.is_null() || self.filename_size == 0 {
                None
            } else {
                let filename_slice = unsafe {
                    std::slice::from_raw_parts(
                        self.p_filename as *const u8,
                        self.filename_size as usize,
                    )
                };
                let str = str::from_utf8(filename_slice)?;
                Some(str.into())
            }
        };

        Ok(ModuleCompileInfoSourceDescriptor { data, filename })
    }
}

/// Module compile info struct for CFFI boundaries
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HyModuleCompileInfo {
    pub s_type: HyStructureType,
    pub p_base_path: *const std::os::raw::c_char,
    pub base_path_size: u32,
    pub p_source_descriptors: *const HyModuleCompileInfoSourceDescriptor,
    pub source_descriptor_count: u32,
    pub flags: HyModuleCompileInfoFlags,
    pub p_next: *mut std::os::raw::c_void,
}

impl HyModuleCompileInfo {
    pub unsafe fn to_module_compile_info(&self) -> anyhow::Result<crate::api::ModuleCompileInfo> {
        if self.s_type != HyStructureType::ModuleCompileInfo {
            return Err(anyhow::anyhow!(
                "Invalid structure type: expected ModuleCompileInfo, got {:?}",
                self.s_type
            ));
        }

        if self.p_source_descriptors.is_null() && self.source_descriptor_count > 0 {
            return Err(anyhow::anyhow!(
                "Source descriptor count is {}, but p_source_descriptors is null",
                self.source_descriptor_count
            ));
        }

        let base_path = if self.p_base_path.is_null() || self.base_path_size == 0 {
            None
        } else {
            let path_slice = unsafe {
                std::slice::from_raw_parts(
                    self.p_base_path as *const u8,
                    self.base_path_size as usize,
                )
            };
            Some(PathBuf::from(str::from_utf8(path_slice)?))
        };

        let mut source_descriptors = Vec::new();
        for i in 0..self.source_descriptor_count {
            let descriptor = unsafe { *self.p_source_descriptors.add(i as usize) };
            source_descriptors.push(descriptor.from_cffi()?);
        }

        Ok(crate::api::ModuleCompileInfo {
            base_path,
            source_descriptors,
            flags: ModuleCompileInfoFlags::from_bits_retain(self.flags),
            ext: unsafe {
                ExtList::from_cffi(self.p_next)
                    .with_context(|| format!("Failed to build ExtList from p_next"))?
            },
        })
    }
}
