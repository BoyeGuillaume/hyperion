//! Inline variable identifiers used across the crate.
//!
//! A compact newtype around `u64` with helpers for display and application.
use strum::{EnumIs, EnumTryAs};

use crate::{
    encoding::{
        EncodableExpr,
        tree::{TreeBuf, TreeBufNodeRef},
    },
    expr::variant::ExprType,
};

/// Identifier for a variable, either internal or external.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIs, EnumTryAs)]
pub enum Variable {
    /// An internal variable, e.g., bound in a lambda abstraction or a quantifier.
    Internal(u32),

    /// An external variable, e.g., a constant in a context or environment.
    External(u32),
}

impl Variable {
    /// Maximum valid id for either internal or external variables (63 bits).
    pub const MAX_VARIABLE_ID: u32 = (1 << 31) - 1;

    /// Create a new internal variable with the given numeric id.
    pub fn raw(&self) -> u32 {
        match self {
            Variable::Internal(id) => {
                debug_assert!(*id <= Self::MAX_VARIABLE_ID);
                (*id << 1) & !1
            }
            Variable::External(id) => {
                debug_assert!(*id <= Self::MAX_VARIABLE_ID);
                (*id << 1) | 1
            }
        }
    }

    /// Create a new variable from a raw numeric id.
    #[inline]
    pub fn new_from_raw(id: u32) -> Self {
        if id & 1 == 0 {
            Variable::Internal(id >> 1)
        } else {
            Variable::External(id >> 1)
        }
    }
}

impl Into<InlineVariable> for Variable {
    fn into(self) -> InlineVariable {
        InlineVariable::new(self)
    }
}

/// Identifier for an inline variable.
///
/// Variables can either represent internal bounds (e.g., in lambda abstractions) or can represent
/// external constants (e.g., in a context or environment). The LSB of the `u32` is reserved to
/// distinguish these two cases;
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineVariable(u32);

impl InlineVariable {
    /// Create from a variable
    pub fn new(v: Variable) -> Self {
        Self(v.raw())
    }

    /// Create a new identifier with the given numeric id.
    pub fn new_from_raw(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw numeric id.
    pub fn raw(&self) -> u32 {
        self.0
    }

    /// Convert to a `Variable`, marking it as internal or external.
    #[inline]
    pub fn to_variable(&self) -> Variable {
        Variable::new_from_raw(self.0)
    }

    /// Return a short printable symbol for small ids.
    ///
    /// For `variant = false`, the range 0..=25 maps to `A..Z`; for `true`, to `a..z`.
    /// Larger ids return `None`.
    pub fn symbol(&self, variant: bool) -> Option<char> {
        let base = if variant { b'A' } else { b'a' };
        if self.0 < 26 {
            Some((base + (self.0 as u8)) as char)
        } else {
            None
        }
    }
}

impl std::fmt::Display for InlineVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(c) = self.symbol(f.alternate()) {
            write!(f, "{}", c)
        } else {
            write!(f, "v{}", self.raw() - 26)
        }
    }
}

/// ======================== Encoding =========================
impl EncodableExpr for InlineVariable {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Variable as u8, Some(self.raw()), &[])
    }
}

impl EncodableExpr for Variable {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Variable as u8, Some(self.raw()), &[])
    }
}
