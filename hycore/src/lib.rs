//! Core runtime, plugin, and specification primitives for the Hyperion engine.
//!
//! The crate exposes a minimal surface area so embedders can construct instances,
//! wire extensions, and author specifications without depending on the binary host.
//! Most consumers will interact with [`base::InstanceContext`] and the modules re-
//! exported below.

pub mod base;
pub mod compiler;
pub mod ext;
pub mod formal;
pub mod magic;
pub mod theorems;
pub mod utils;

pub extern crate chrono;
pub extern crate inventory;

#[macro_export]
macro_rules! register {
    (plugin $ty:ty) => {
        crate::register_plugin!($ty);
    };
    (theorem_inference_strategy $ty:ty) => {
        $crate::register_theorem_inference_strategy!($ty);
    };
}
