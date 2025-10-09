//! Discriminant for encoded expression nodes.
//!
//! Role
//! - Stable mapping from a single-byte opcode to a high-level constructor.
//! - Used during decoding to route to the appropriate `ExprView` variant.
use strum::{EnumIter, FromRepr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, FromRepr)]
#[repr(u8)]
pub enum ExprType {
    // Constant expr
    Bool,
    Omega,
    True,
    False,
    Never,

    // Unary expr
    Not,
    Powerset,

    // Binary expr
    And,
    Or,
    Implies,
    Iff,
    Equal,
    Lambda,
    Call,
    Tuple,
    Forall,
    Exists,

    // Ternary expr
    If,

    // Misc
    Variable,
}
