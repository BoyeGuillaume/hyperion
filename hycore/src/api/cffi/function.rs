#![allow(unused_variables)]
use semver::Version;

use crate::{
    api::cffi::{r#struct::*, *},
    hydebug,
    instance::Instance,
};

struct LastError {
    message: String,
    backtrace: String,
}
static LAST_ERROR: std::sync::Mutex<Option<LastError>> = std::sync::Mutex::new(None);

fn return_error(error: anyhow::Error) -> std::ffi::c_int {
    let backtrace = error.backtrace().to_string();
    let message = error.to_string();
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
