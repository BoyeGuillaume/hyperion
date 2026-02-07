//! Define external symbols and linkage information for modules.
//!
//! This module provides structures to represent external symbols
//! (functions and global variables) and their linkage types within a
//! module. It allows specifying whether a symbol is defined within the
//! module or is an external reference.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{modules::CallingConvention, types::Typeref};

/// Defines an externally linked function
///
/// This struct represents a function that is defined outside the current module,
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct ExternalFunction {
    /// Unique identifier for the external function. This is used internally to
    /// reference the function within the module.
    pub uuid: Uuid,

    /// The name of the external function as it appears in the linking context.
    pub name: String,

    /// The calling convention used by the external function.
    pub cconv: CallingConvention,

    /// The parameter types of the external function.
    pub param_types: Vec<Typeref>,

    /// The return type of the external function. `None` indicates a `void` return type.
    pub return_type: Option<Typeref>,
}

impl ExternalFunction {
    pub fn iter_referenced_typerefs(&self) -> impl Iterator<Item = &Typeref> {
        self.param_types.iter().chain(self.return_type.iter())
    }

    pub fn iter_referenced_typerefs_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        self.param_types
            .iter_mut()
            .chain(self.return_type.iter_mut())
    }

    pub fn remap_types(&mut self, mapping: impl Fn(&Typeref) -> Option<Typeref>) {
        for param_type in self.param_types.iter_mut() {
            if let Some(new_type) = mapping(param_type) {
                *param_type = new_type;
            }
        }
        if let Some(ret_type) = &mut self.return_type
            && let Some(new_type) = mapping(ret_type)
        {
            *ret_type = new_type;
        }
    }
}

/// A reference to either a function (internal or external) or a global variable
///
/// Internal functions are defined within the current module, while external
/// functions are declared but defined outside the module. Typically external are
/// functions used to interface or link with other modules or libraries.
///
/// Globals are constant that can be accessed across the module.
///
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Pointer(pub Uuid);
