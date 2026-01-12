//! Core runtime, plugin, and specification primitives for the Hyperion engine.
//!
//! The crate exposes a minimal surface area so embedders can construct instances,
//! wire extensions, and author specifications without depending on the binary host.
//! Most consumers will interact with [`base::InstanceContext`] and the modules re-
//! exported below.

pub mod base;
pub mod ext;
pub mod magic;
pub mod provers;
pub mod specifications;
#[cfg(any(test, feature = "test-utils"))]
pub mod tests_utils;
pub mod utils;

pub extern crate chrono;
pub extern crate inventory;
