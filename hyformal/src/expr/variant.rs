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

impl ExprType {
    /// Number of children this constructor has.
    #[inline]
    pub fn arity(self) -> u8 {
        use ExprType::*;
        match self {
            Bool | Omega | True | False | Never | Variable => 0,
            Not | Powerset => 1,
            And | Or | Implies | Iff | Equal | Lambda | Call | Tuple | Forall | Exists => 2,
            If => 3,
        }
    }

    /// Whether nodes of this constructor carry a 32-bit payload in the buffer.
    #[inline]
    pub fn has_data(self) -> bool {
        matches!(
            self,
            ExprType::Variable | ExprType::Forall | ExprType::Exists
        )
    }
}
