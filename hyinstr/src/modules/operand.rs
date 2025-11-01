//! Shared operand types for instructions.
//!
//! An instruction operand can be a reference to another SSA value (`Reg`),
//! an immediate constant (`Imm`) or a code label (`Lbl`).
use crate::consts::AnyConst;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// SSA value identifier used to name the destination or reference another
/// instruction's result.
pub type Name = u32;

/// Instruction operand.
///
/// - `Reg(Name)`: reference to a previously defined SSA value
/// - `Imm(AnyConst)`: immediate literal (integer or floating‑point)
/// - `Lbl(())`: code label placeholder (reserved for control‑flow uses)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Operand {
    Reg(Name),
    Imm(AnyConst),
    Lbl(()),
}
