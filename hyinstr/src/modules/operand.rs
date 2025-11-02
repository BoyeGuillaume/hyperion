//! Shared operand types for instructions.
//!
//! An instruction operand can be a reference to another SSA value (`Reg`),
//! an immediate constant (`Imm`) or a code label (`Lbl`).
use crate::consts::AnyConst;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SSA value identifier used to name the destination or reference another
/// instruction's result.
pub type Name = u32;

/// Represents a code label used as a target for control‑flow instructions (besides invokes).
///
/// Notice that in hyperion, labels and control-flow may not cross function boundaries. Thus,
/// labels are only valid within the function they are defined in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Label(pub(super) Uuid);

/// Instruction operand.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Operand {
    /// Reference to a previously defined SSA value.
    Reg(Name),
    /// Immediate literal (integer or floating‑point).
    Imm(AnyConst),
    /// Code label (used for control‑flow).
    Lbl(Label),
}
