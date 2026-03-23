use bevy_ecs::entity::Entity;
use semver::Version;

use crate::{
    api::{
        cffi::{r#struct::*, *},
        hy_compile_module,
    },
    hydebug,
    instance::{Instance, core::ModuleHandle},
};

struct LastError {
    message: String,
    backtrace: String,
}
static LAST_ERROR: std::sync::Mutex<Option<LastError>> = std::sync::Mutex::new(None);

fn return_error(error: anyhow::Error) -> std::ffi::c_int {
    let backtrace = error.backtrace().to_string();
    let message = format!("{:?}", error);
    let last_error = LastError { message, backtrace };
    *LAST_ERROR.lock().unwrap() = Some(last_error);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyGetVersionInfo(p_version_info: *mut HyVersionInfo) {
    if p_version_info.is_null() {
        return;
    }

    let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let version_info = HyVersionInfo {
        major: version.major.min(u16::MAX as u64) as u16,
        minor: version.minor.min(u16::MAX as u64) as u16,
        patch: version.patch.min(u16::MAX as u64) as u16,
    };

    unsafe {
        *p_version_info = version_info;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyGetLastError(
    p_buffer: *mut std::os::raw::c_char,
    buffer_size: u32,
    p_backtrace_buffer: *mut std::os::raw::c_char,
    backtrace_buffer_size: u32,
) -> std::ffi::c_int {
    let last_error = LAST_ERROR.lock().unwrap();
    if let Some(last_error) = &*last_error {
        // Fill the buffer if it's large enough
        if !p_buffer.is_null() && buffer_size > 0 {
            let error_message_bytes = last_error.message.as_bytes();
            let copy_size = std::cmp::min(error_message_bytes.len(), buffer_size as usize - 1);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    error_message_bytes.as_ptr(),
                    p_buffer as *mut u8,
                    copy_size,
                );
                *p_buffer.add(copy_size) = 0; // Null-terminate the string
            }
        }

        // Fill the backtrace buffer if it's large enough
        if !p_backtrace_buffer.is_null() && backtrace_buffer_size > 0 {
            let backtrace_bytes = last_error.backtrace.as_bytes();
            let copy_size =
                std::cmp::min(backtrace_bytes.len(), backtrace_buffer_size as usize - 1);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    backtrace_bytes.as_ptr(),
                    p_backtrace_buffer as *mut u8,
                    copy_size,
                );
                *p_backtrace_buffer.add(copy_size) = 0; // Null-terminate the string
            }
        }

        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyCreateInstance(
    p_create_info: *const HyInstanceCreateInfo,
    pp_instance: *mut *mut HyInstance,
) -> std::ffi::c_int {
    // Convert the p_create_info pointer to a Rust reference
    let create_info = unsafe {
        if p_create_info.is_null() {
            return return_error(anyhow::anyhow!(
                "Instance create info pointer cannot be null"
            ));
        }

        if pp_instance.is_null() {
            return return_error(anyhow::anyhow!(
                "Cannot create instance: output instance pointer cannot be null"
            ));
        }

        &*p_create_info
    };

    // Convert the create info to the internal Rust representation
    let create_info = match unsafe { create_info.to_instance_create_info() } {
        Ok(info) => info,
        Err(e) => return return_error(e),
    };

    // Create the instance (this is where you would call your actual instance creation logic)
    let instance = match Instance::new(create_info) {
        Ok(instance) => instance,
        Err(e) => return return_error(e),
    };

    // Allocate a new Box containing the instance and return a pointer to it
    let boxed_instance = Box::new(instance);

    unsafe {
        let instance = Box::into_raw(boxed_instance);
        *pp_instance = instance as *mut HyInstance;
        hydebug!(
            *instance; "New instance object created at address {:p}",
            *pp_instance
        );
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyDestroyInstance(p_instance: *mut HyInstance) {
    if p_instance.is_null() {
        return;
    }

    // SAFETY: We must ensure that the pointer is valid and was allocated by this library. We also need to ensure
    // that it isn't concurrently used, nor was it already freed.
    let instance = unsafe { Box::from_raw(p_instance as *mut Instance) };
    hydebug!(
        *instance; "Instance object at address {:p} is being destroyed",
        p_instance
    );

    // Instance will be dropped here when it goes out of scope
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyCompileModule(
    p_instance: *mut HyInstance,
    p_compile_info: *const HyModuleCompileInfo,
    pp_output_buffer: *mut *mut u8,
    p_output_buffer_size: *mut u32,
) -> std::ffi::c_int {
    if p_instance.is_null() {
        return return_error(anyhow::anyhow!("Instance pointer cannot be null"));
    }

    if p_compile_info.is_null() {
        return return_error(anyhow::anyhow!("Compile info pointer cannot be null"));
    }

    if pp_output_buffer.is_null() || p_output_buffer_size.is_null() {
        return return_error(anyhow::anyhow!("Output buffer pointer cannot be null"));
    }

    // Convert the compile info to the internal Rust representation
    let compile_info = unsafe { &*p_compile_info };
    let compile_info = match unsafe { compile_info.to_module_compile_info() } {
        Ok(info) => info,
        Err(e) => return return_error(e),
    };
    let instance: &Instance = unsafe { &*(p_instance as *mut Instance) };

    // Compile here
    match hy_compile_module(instance, compile_info) {
        Ok(compiled_module) => {
            let buffer_size = compiled_module.len() as u32;
            let buffer = unsafe { libc::malloc(buffer_size as usize) } as *mut u8;
            if buffer.is_null() {
                return return_error(anyhow::anyhow!("Failed to allocate output buffer"));
            }

            unsafe {
                std::ptr::copy_nonoverlapping(
                    compiled_module.as_ptr(),
                    buffer,
                    buffer_size as usize,
                );
                *pp_output_buffer = buffer;
                *p_output_buffer_size = buffer_size;
            }
            0
        }
        Err(e) => return_error(e),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyFreeCompiledModuleBuffer(p_buffer: *mut u8) {
    if p_buffer.is_null() {
        return;
    }

    unsafe {
        libc::free(p_buffer as *mut std::ffi::c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyLoadCompiledModule(
    p_instance: *mut HyInstance,
    p_module_buffer: *const u8,
    module_buffer_size: u32,
    p_module: *mut HyModule,
) -> std::ffi::c_int {
    if p_instance.is_null() {
        return return_error(anyhow::anyhow!("Instance pointer cannot be null"));
    }

    if p_module.is_null() {
        return return_error(anyhow::anyhow!("Module output pointer cannot be null"));
    }

    if p_module_buffer.is_null() || module_buffer_size == 0 {
        return return_error(anyhow::anyhow!(
            "Module buffer pointer cannot be null or size cannot be zero"
        ));
    }

    let instance: &mut Instance = unsafe { &mut *(p_instance as *mut Instance) };
    let module_buffer =
        unsafe { std::slice::from_raw_parts(p_module_buffer, module_buffer_size as usize) };

    match crate::api::hy_load_compiled_module(instance, module_buffer) {
        Ok(entity) => {
            // If system sizeof pointer is larger than sizeof u64
            let entityptr: &mut u64 = unsafe { &mut *p_module };
            *entityptr = entity.get().to_bits();

            0
        }
        Err(e) => return_error(e),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyDestroyModule(
    p_instance: *mut HyInstance,
    module: HyModule,
) -> std::ffi::c_int {
    if p_instance.is_null() {
        return return_error(anyhow::anyhow!("Instance pointer cannot be null"));
    }

    let instance: &mut Instance = unsafe { &mut *(p_instance as *mut Instance) };
    let module_handle = ModuleHandle(Entity::from_bits(module as u64));

    match crate::api::hy_destroy_module(instance, module_handle) {
        Ok(()) => 0,
        Err(e) => return_error(e),
    }
}
