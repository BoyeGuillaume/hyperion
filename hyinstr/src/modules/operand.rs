//! Shared operand types for instructions.
//!
//! An instruction operand can be a reference to another SSA value (`Reg`),
//! an immediate constant (`Imm`), a type-tagged undefined value (`Undef`) or a
//! code label (`Lbl`).
use std::fmt::Debug;

use crate::consts::AnyConst;
use crate::types::Typeref;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

/// SSA value identifier used to name the destination or reference another
/// instruction's result.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Name(pub u32);

impl std::ops::Add<u32> for Name {
    type Output = Name;

    fn add(self, rhs: u32) -> Self::Output {
        Name(self.0 + rhs)
    }
}

impl std::ops::AddAssign<u32> for Name {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
    }
}

impl Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
    }
}

/// Represents a meta operand used internally in attributes and properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MetaLabel(pub u32);

/// Represents a code label used as a target for control‑flow instructions (besides invokes).
///
/// Notice that in hyperion, labels and control-flow may not cross function boundaries. Thus,
/// labels are only valid within the function they are defined in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Label(pub u32);

impl Label {
    /// Reserved entry label used for the first basic block.
    pub const NIL: Label = Label(0);

    /// Returns true if this is the "nil" label (i.e., label 0).
    /// This label is reserved as the function entry label and should always be present.
    pub fn is_nil(&self) -> bool {
        self == &Label::NIL
    }
}

/// Instruction operand.
#[derive(Clone, Debug, PartialEq, Eq, Hash, EnumIs, EnumTryAs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum Operand {
    /// Reference to a previously defined SSA value.
    Reg(Name),
    /// Immediate literal (integer or floating‑point).
    Imm(AnyConst),
    /// Type-tagged undefined SSA value.
    Undef(Typeref),
}
