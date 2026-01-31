//! hyinstr — a lightweight instruction IR
//!
//! This crate provides a compact, serializable intermediate representation (IR)
//! for low-level programs. It focuses on simple, explicit instruction shapes
//! (integer, floating‑point, memory, and control flow), a small set of shared
//! operand and symbol types, and a minimal type system backed by a registry.
//!
//! Key building blocks:
//! - `modules`: instruction definitions and shared operands/symbols
//! - `types`: primitive and aggregate types with a registry (`TypeRegistry`)
//! - `consts`: immediate constants usable in instructions
//!
//! You typically construct instructions directly by populating their public
//! fields, then place them in basic blocks (`BasicBlock`) and functions
//! (`Function`). A `Module` groups functions and external symbols.
pub mod analysis;
pub mod attached;
pub mod consts;
pub mod modules;
pub mod types;
pub mod utils;
