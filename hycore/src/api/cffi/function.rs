#![allow(unused_variables)]
use crate::api::cffi::{r#struct::*, *};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyCreateInstance(
    p_create_info: *const HyInstanceCreateInfo,
    pp_instance: *mut *mut HyInstance,
) -> std::ffi::c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyDestroyInstance(p_instance: *mut HyInstance) {}
