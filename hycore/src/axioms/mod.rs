//! Attribute (property) data structures for functions and modules.
//!
//! This module contains the foundational immutable (and append-only) metadata
//! structures used to associate logical properties with IR modules and
//! functions. Properties are stored separately from the executable IR and are
//! intended to support reasoning, verification, optimization hints, and
//! downstream tooling.
//!
//! ## Submodules
//! * [`function`] – Structures describing intra-function logical artifacts:
//!   assertions, assumptions, internal SSA-like intermediary instructions,
//!   behavior cases, and indexing helpers.
//! * [`modules`] – A light wrapper binding a compiled/parsed [`hyinstr::modules::Module`]
//!   to a map of function UUIDs and their corresponding [`function::FunctionMetadata`].
//!
//! All data types are designed to be serializable (or trivially made so) and
//! rely on stable indexing to permit pause/resume of derivation steps without
//! recomputation.
pub mod function;
pub mod modules;
