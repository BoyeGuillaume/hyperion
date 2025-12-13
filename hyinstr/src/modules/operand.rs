//! Shared operand types for instructions.
//!
//! An instruction operand can be a reference to another SSA value (`Reg`),
//! an immediate constant (`Imm`) or a code label (`Lbl`).
use crate::consts::AnyConst;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

/// SSA value identifier used to name the destination or reference another
/// instruction's result.
pub type Name = u32;

/// Represents a meta‑operand used internally in attributes/properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MetaLabel(pub u32);

/// Represents a code label used as a target for control‑flow instructions (besides invokes).
///
/// Notice that in hyperion, labels and control-flow may not cross function boundaries. Thus,
/// labels are only valid within the function they are defined in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Label(pub u32);

impl Label {
    pub const NIL: Label = Label(0);

    /// Returns true if this is the "nil" label (i.e., label 0).
    ///
    /// This label is reserved as the 'function entry' label. It should always be present.
    /// Returns true if this is the "nil" label (i.e., label 0).
    ///
    pub fn is_nil(&self) -> bool {
        self == &Label::NIL
    }
}

/// Instruction operand.
#[derive(Clone, Debug, PartialEq, Eq, Hash, EnumIs, EnumTryAs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Operand {
    /// Reference to a previously defined SSA value.
    Reg(Name),
    /// Immediate literal (integer or floating‑point).
    Imm(AnyConst),
    /// Meta operand (only used internally in attributes/properties)
    ///
    /// Notice: Meta operands should not appear in regular instructions and
    /// is prohibeted to appear in well-formed modules. Should only be used
    /// in attributes/properties.
    Meta(MetaLabel),
}
