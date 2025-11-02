//! Shared operand types for instructions.
//!
//! An instruction operand can be a reference to another SSA value (`Reg`),
//! an immediate constant (`Imm`) or a code label (`Lbl`).
use crate::{consts::AnyConst, modules::Module};
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

impl Operand {
    pub fn fmt<'a>(&'a self, module: Option<&'a Module>) -> impl std::fmt::Display + 'a {
        pub struct Fmt<'a> {
            operand: &'a Operand,
            module: Option<&'a Module>,
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.operand {
                    Operand::Reg(name) => write!(f, "%{}", name),
                    Operand::Imm(constant) => write!(f, "{}", constant.fmt(self.module)),
                    Operand::Lbl(label) => write!(f, "label_%{}", label.0),
                }
            }
        }

        Fmt {
            operand: self,
            module,
        }
    }
}
