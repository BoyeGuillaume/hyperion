//! Constant values
//!
//! Literal values used as immediate operands in instructions. Both integer
//! and floating‑point constants are supported, with arbitrary precision types
//! where appropriate.
use crate::consts::{fp::FConst, int::IConst};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod fp;
pub mod int;

/// A constant value (integer or floating‑point) usable as an immediate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnyConst {
    Int(IConst),
    Float(FConst),
}
