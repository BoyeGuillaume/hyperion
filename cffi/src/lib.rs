use std::{
    ffi::CString,
    os::raw::{c_char, c_void},
    sync::{Arc, Weak},
};

use hycore::{
    base::{
        api::{ModuleSourceInfo, ModuleSourceType, VersionInfo},
        InstanceContext, ModuleKey,
    },
    ext::hylog::LogLevelEXT,
    hywarn,
    utils::{error::HyErrorType, opaque::OpaqueList},
};
use strum::FromRepr;

pub struct HyInstance(Arc<InstanceContext>);
pub struct HyModule(ModuleKey, Weak<InstanceContext>);

/// cbindgen:rename-all=ScreamingSnakeCase
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HyResult {
    HyResultSuccess,
    HyResultInvalidPointer,
    HyResultIoError,
    HyResultOutOfMemory,
    HyResultManifestParseError,
    HyResultUnknown,
    HyResultPluginNotFound,
    HyResultUtf8Error,
    HyResultInstrError,
    HyResultKeyNotFound,
    HyResultStructureTypeMismatch,
}

/// cbindgen:rename-all=ScreamingSnakeCase
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum HyLogLevelEXT {
    HyLogLevelTrace = 0,
    HyLogLevelDebug = 1,
    HyLogLevelInfo = 2,
    HyLogLevelWarn = 3,
    HyLogLevelError = 4,
}

/// cbindgen:rename-all=ScreamingSnakeCase
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HyStructureType {
    HyStructureTypeInstanceCreateInfo,
    HyStructureTypeApplicationInfo,
    HyStructureTypeModuleCompileInfo,
    HyStructureTypeModuleSourceInfo,
    HyStructureTypeLogCreateInfoEXT = 0x10000000,
}

impl Into<HyLogLevelEXT> for LogLevelEXT {
    fn into(self) -> HyLogLevelEXT {
        HyLogLevelEXT::from_repr(self as u32).unwrap()
    }
}

impl From<HyLogLevelEXT> for LogLevelEXT {
    fn from(value: HyLogLevelEXT) -> Self {
        LogLevelEXT::from_repr(value as u32).unwrap()
    }
}

impl From<hycore::utils::error::HyError> for HyResult {
    fn from(value: hycore::utils::error::HyError) -> Self {
        let value_type: HyErrorType = value.into();
        match value_type {
            HyErrorType::IoError => HyResult::HyResultIoError,
            HyErrorType::ManifestParseError => HyResult::HyResultManifestParseError,
            HyErrorType::PluginNotFound => HyResult::HyResultPluginNotFound,
            HyErrorType::Utf8Error => HyResult::HyResultUtf8Error,
            HyErrorType::HyInstrError => HyResult::HyResultInstrError,
            HyErrorType::KeyNotFound => HyResult::HyResultKeyNotFound,
            HyErrorType::Unknown => HyResult::HyResultUnknown,
        }
    }
}

// Version info matching hycore::base::api::VersionInfo
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HyVersionInfo {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Into<VersionInfo> for HyVersionInfo {
    fn into(self) -> VersionInfo {
        VersionInfo {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
        }
    }
}

// Constants mirroring hycore::base::api::ModuleSourceType
// pub const HY_MODULE_SOURCE_ASSEMBLY: u32 = hycore::base::api::ModuleSourceType::Assembly as u32;

/// cbindgen:rename-all=CamelCase
#[repr(C)]
pub struct HyApplicationInfo {
    pub s_type: HyStructureType,
    pub application_version: HyVersionInfo,
    pub p_application_name: *const c_char,
    pub engine_version: HyVersionInfo,
    pub p_engine_name: *const c_char,
}

/// cbindgen:rename-all=CamelCase
#[repr(C)]
pub struct HyInstanceCreateInfo {
    pub s_type: HyStructureType,
    pub p_application_info: *const HyApplicationInfo,
    pub pp_enabled_extensions: *const *const c_char,
    pub enabled_extensions_count: u32,
    pub node_id: u32,
    pub p_next: *mut c_void, // opaque, must be null for now
}

/// cbindgen:rename-all=CamelCase
#[repr(C)]
pub struct HyLogMessageEXT {
    pub level: HyLogLevelEXT,
    pub time_stamp: i64, // Unix timestamp
    pub message: *const c_char,
    pub module: *const c_char,
    pub file: *const c_char,
    pub line: u32,
    pub thread_name: *const c_char,
    pub p_next: *mut c_void, // opaque, must be null for now
}

#[allow(non_camel_case_types)]
pub type HyLogCallback_PFN = extern "C" fn(message: *mut HyLogMessageEXT);

/// cbindgen:rename-all=ScreamingSnakeCase
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum HyModuleSourceType {
    HyModuleSourceTypeAssembly,
}

impl Into<ModuleSourceType> for HyModuleSourceType {
    fn into(self) -> ModuleSourceType {
        match self {
            HyModuleSourceType::HyModuleSourceTypeAssembly => ModuleSourceType::Assembly,
        }
    }
}

/// cbindgen:rename-all=CamelCase
#[repr(C)]
pub struct HyModuleSourceInfo {
    pub s_type: HyStructureType,
    pub source_type: HyModuleSourceType,
    pub filename: *const c_char, // nullable
    pub data: *const u8,
}

/// cbindgen:rename-all=CamelCase
#[repr(C)]
pub struct HyModuleCompileInfo {
    pub s_type: HyStructureType,
    pub pp_sources: *const *const HyModuleSourceInfo,
    pub sources_count: u32,
}

/// cbindgen:rename-all=CamelCase
#[repr(C)]
pub struct HyLogCreateInfoEXT {
    pub s_type: HyStructureType,
    pub level: HyLogLevelEXT,
    pub callback: HyLogCallback_PFN,
    pub p_next: *mut c_void, // opaque, must be null for now
}

pub unsafe fn verify_structure_type<T>(element: *const T, expected: HyStructureType) -> bool {
    if element.is_null() {
        return false;
    }
    let s_type_ptr = element as *const HyStructureType;
    let s_type = unsafe { *s_type_ptr };
    s_type == expected
}

pub unsafe fn convert_opaque_list_from_next(
    mut p_next: *const c_void,
) -> Result<OpaqueList, HyResult> {
    let mut list = vec![];

    while p_next != std::ptr::null() {
        // Read p_next sType
        let s_type = unsafe {
            let s_type_ptr = p_next as *const HyStructureType;
            *s_type_ptr
        };

        match s_type {
            HyStructureType::HyStructureTypeLogCreateInfoEXT => {
                let log_create_info = unsafe {
                    let ptr = p_next as *const HyLogCreateInfoEXT;
                    &*ptr
                };

                let level: LogLevelEXT = log_create_info.level.into();
                let callback = log_create_info.callback;

                let create_info = hycore::ext::hylog::LogCreateInfoEXT {
                    level,
                    callback: hycore::ext::hylog::LogCallbackEXT(Box::new(move |msg| {
                        let message = CString::new(msg.message.clone()).unwrap_or_default();
                        let module = CString::new(msg.module.clone()).unwrap_or_default();
                        let file =
                            CString::new(msg.file.clone().unwrap_or_default()).unwrap_or_default();
                        let thread_name = CString::new(msg.thread_name.clone().unwrap_or_default())
                            .unwrap_or_default();

                        let mut message = HyLogMessageEXT {
                            level: msg.level.into(),
                            time_stamp: msg.timepoint.and_utc().timestamp(),
                            message: message.as_ptr() as *const c_char,
                            module: module.as_ptr() as *const c_char,
                            file: file.as_ptr() as *const c_char,
                            line: msg.line.unwrap_or(0),
                            thread_name: thread_name.as_ptr() as *const c_char,
                            p_next: std::ptr::null_mut(),
                        };
                        let message_ptr: *mut HyLogMessageEXT = &mut message;
                        callback(message_ptr);
                    })),
                };
                list.push(Box::new(create_info) as Box<dyn hycore::utils::opaque::OpaqueObject>);
                p_next = log_create_info.p_next;
            }
            _ => {
                return Err(HyResult::HyResultStructureTypeMismatch);
            }
        }
    }

    Ok(OpaqueList(list))
}

/// Retrieves information about the version of the Hycore library.
///
/// # Safety
/// - The `pVersionInfo` pointer must be a valid, non-null pointer to a `HyVersionInfo` struct.
///cbindgen:rename-all=CamelCase
#[no_mangle]
pub extern "C" fn hyGetVersionInfo(p_version_info: *mut HyVersionInfo) {
    if p_version_info.is_null() {
        return;
    }
    let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

    unsafe {
        *p_version_info = HyVersionInfo {
            major: version.major as u16,
            minor: version.minor as u16,
            patch: version.patch as u16,
        };
    }
}

/// Create an instance of the Hycore library.
///
/// # Safety
/// - The `pInstanceCreateInfo` pointer must be a valid, non-null pointer to a `HyInstanceCreateInfo` struct.
/// - The `pInstance` pointer must be a valid, non-null pointer to a pointer to `HyInstance`.
///cbindgen:rename-all=CamelCase
#[no_mangle]
pub extern "C" fn hyCreateInstance(
    p_instance_create_info: *const HyInstanceCreateInfo,
    p_instance: *mut *mut HyInstance,
) -> HyResult {
    if p_instance_create_info.is_null() || p_instance.is_null() {
        return HyResult::HyResultInvalidPointer;
    }

    if unsafe {
        !verify_structure_type(
            p_instance_create_info,
            HyStructureType::HyStructureTypeInstanceCreateInfo,
        )
    } {
        return HyResult::HyResultStructureTypeMismatch;
    }

    // Convert and validate input info
    let info_ref = unsafe { &*p_instance_create_info };
    if info_ref.p_application_info.is_null() {
        return HyResult::HyResultInvalidPointer;
    }
    if unsafe {
        !verify_structure_type(
            info_ref.p_application_info,
            HyStructureType::HyStructureTypeApplicationInfo,
        )
    } {
        return HyResult::HyResultStructureTypeMismatch;
    }
    let p_application_info = unsafe { &*info_ref.p_application_info };

    let app_name = if p_application_info.p_application_name.is_null() {
        String::new()
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(p_application_info.p_application_name)
                .to_string_lossy()
                .into_owned()
        }
    };

    let engine_name = if p_application_info.p_engine_name.is_null() {
        String::new()
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(p_application_info.p_engine_name)
                .to_string_lossy()
                .into_owned()
        }
    };

    let application_info = hycore::base::api::ApplicationInfo {
        application_version: p_application_info.application_version.into(),
        application_name: app_name,
        engine_version: p_application_info.engine_version.into(),
        engine_name,
    };

    // Convert and validated enabled extensions
    let enabled_extensions =
        if info_ref.pp_enabled_extensions.is_null() || info_ref.enabled_extensions_count == 0 {
            Vec::new()
        } else {
            let slice = unsafe {
                std::slice::from_raw_parts(
                    info_ref.pp_enabled_extensions,
                    info_ref.enabled_extensions_count as usize,
                )
            };
            slice
                .iter()
                .map(|&p| {
                    if p.is_null() {
                        String::new()
                    } else {
                        unsafe { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
                    }
                })
                .collect()
        };

    // Convert opaque list from pNext
    let opaque_list = match unsafe { convert_opaque_list_from_next(info_ref.p_next) } {
        Ok(list) => list,
        Err(err) => return err,
    };

    let create_info = hycore::base::api::InstanceCreateInfo {
        application_info,
        enabled_extensions: enabled_extensions,
        node_id: info_ref.node_id,
        ext: opaque_list,
    };
    match hycore::base::api::create_instance(create_info) {
        Ok(ctx) => {
            let boxed = Box::new(HyInstance(ctx));
            unsafe {
                *p_instance = Box::into_raw(boxed);
            }
            HyResult::HyResultSuccess
        }
        Err(err) => err.into(),
    }
}

/// Destroys an instance created by `hyCreateInstance`.
///
/// # Safety
/// - The `instance` pointer must be a valid, non-null pointer to a `HyInstance`.
///cbindgen:rename-all=CamelCase
#[no_mangle]
pub extern "C" fn hyDestroyInstance(instance: *mut HyInstance) {
    if instance.is_null() {
        return;
    }
    unsafe {
        let instance = Box::from_raw(instance);

        /* Check if there are any modules to remove */
        let instance_ref = &instance.0;
        let module_keys: Vec<ModuleKey> = instance_ref.modules.read().keys().collect();
        if module_keys.len() > 0 {
            hywarn!(
                instance_ref,
                "Some modules were not destroyed before instance destruction. Did you forget to call hyDestroyModule on them? Those modules are still loaded {:?}",
                module_keys
            );
        }

        drop(instance);
    }
}

/// Compile module sources into a binary format.
///
/// # Safety
/// - The `instance` pointer must be a valid, non-null pointer to a `HyInstance`.
/// - The `pModuleCompileInfo` pointer must be a valid, non-null pointer to a `HyModuleCompileInfo`.
/// - The `ppDataPtr` and `pDataLen` pointers must be valid, non-null pointers to receive the output data. The caller is responsible for freeing the allocated data using `libc::free`.
///
///cbindgen:rename-all=CamelCase
#[no_mangle]
pub extern "C" fn hyCompileModule(
    instance: *const HyInstance,
    p_module_compile_info: *const HyModuleCompileInfo,
    pp_data_ptr: *mut *mut u8,
    p_data_len: *mut u32,
) -> HyResult {
    if instance.is_null()
        || p_module_compile_info.is_null()
        || pp_data_ptr.is_null()
        || p_data_len.is_null()
    {
        return HyResult::HyResultInvalidPointer;
    }

    // Convert and validate input info
    let inst = unsafe { &*instance };

    // Convert compile info
    if !unsafe {
        verify_structure_type(
            p_module_compile_info,
            HyStructureType::HyStructureTypeModuleCompileInfo,
        )
    } {
        return HyResult::HyResultStructureTypeMismatch;
    }
    let info_ref = unsafe { &*p_module_compile_info };

    // Convert sources
    let sources = if info_ref.pp_sources.is_null() || info_ref.sources_count == 0 {
        Vec::new()
    } else {
        let slice = unsafe {
            std::slice::from_raw_parts(info_ref.pp_sources, info_ref.sources_count as usize)
        };
        let mut sources_vec = Vec::with_capacity(slice.len());
        for &source_ptr in slice {
            if source_ptr.is_null() {
                return HyResult::HyResultInvalidPointer;
            }
            if !unsafe {
                verify_structure_type(source_ptr, HyStructureType::HyStructureTypeModuleSourceInfo)
            } {
                return HyResult::HyResultStructureTypeMismatch;
            }
            let source_ref = unsafe { &*source_ptr };
            let filename = if source_ref.filename.is_null() {
                None
            } else {
                Some(unsafe {
                    std::ffi::CStr::from_ptr(source_ref.filename)
                        .to_string_lossy()
                        .into_owned()
                })
            };
            let data = if source_ref.data.is_null() {
                String::new()
            } else {
                // For simplicity, assume data is null-terminated string
                unsafe {
                    std::ffi::CStr::from_ptr(source_ref.data as *const c_char)
                        .to_string_lossy()
                        .into_owned()
                }
            };
            let source_type: ModuleSourceType = source_ref.source_type.into();
            sources_vec.push(ModuleSourceInfo {
                source_type,
                filename,
                data,
            });
        }
        sources_vec
    };

    // Create compile info
    let compile_info = hycore::base::api::ModuleCompileInfo { sources };

    // Compile sources
    match hycore::base::api::compile_sources(&inst.0, compile_info) {
        Ok(buf) => {
            let len = buf.len() as usize;
            if len >= u32::MAX as usize {
                return HyResult::HyResultOutOfMemory;
            }

            unsafe {
                let ptr = libc::malloc(len) as *mut u8;
                if ptr.is_null() {
                    return HyResult::HyResultOutOfMemory;
                }
                std::ptr::copy_nonoverlapping(buf.as_ptr(), ptr, len);
                *pp_data_ptr = ptr;
                *p_data_len = len as u32;
            }

            HyResult::HyResultSuccess
        }
        Err(err) => err.into(),
    }
}

/// Loads a compiled module from binary data.
///
/// # Safety
/// - The `instance` pointer must be a valid, non-null pointer to a `HyInstance`.
/// - The `pDataPtr` pointer must be a valid, non-null pointer to the compiled module data.
/// - The `dataLen` must be the correct length of the compiled module data.
/// - The `pModule` pointer must be a valid, non-null pointer to receive the created `HyModule`.
///cbindgen:rename-all=CamelCase
#[no_mangle]
pub extern "C" fn hyLoadModule(
    instance: *const HyInstance,
    p_data_ptr: *const u8,
    data_len: u32,
    p_module: *mut *mut HyModule,
) -> HyResult {
    if instance.is_null() || p_data_ptr.is_null() || p_module.is_null() {
        return HyResult::HyResultInvalidPointer;
    }

    let inst = unsafe { &*instance };
    let data = unsafe { std::slice::from_raw_parts(p_data_ptr, data_len as usize) };
    match hycore::base::api::load_module(&inst.0, data) {
        Ok(module_key) => {
            let boxed = Box::new(HyModule(module_key, Arc::downgrade(&inst.0)));
            unsafe {
                *p_module = Box::into_raw(boxed);
            }
            HyResult::HyResultSuccess
        }
        Err(err) => err.into(),
    }
}

/// Destroys a module loaded by `hyLoadModule`.
///
/// # Safety
/// - The `module` pointer must be a valid, non-null pointer to a `HyModule`.
///cbindgen:rename-all=CamelCase
#[no_mangle]
pub extern "C" fn hyDestroyModule(module: *mut HyModule) {
    if module.is_null() {
        return;
    }
    unsafe {
        let boxed_module = Box::from_raw(module);
        if let Some(instance) = boxed_module.1.upgrade() {
            let _ = instance.remove_module_by_key(boxed_module.0);
        }
        drop(boxed_module);
    }
}
