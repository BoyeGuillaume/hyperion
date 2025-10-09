//! Inline variable identifiers used across the crate.
//!
//! Role
//! - Provide compact identifiers for variables used in lambda/quantifiers or external contexts.
//! - Encode to/from expression nodes without allocation.
use strum::{EnumIs, EnumTryAs};

use crate::{
    encoding::{
        EncodableExpr,
        tree::{TreeBuf, TreeBufNodeRef},
    },
    expr::{AnyExpr, Expr, expr_sealed, variant::ExprType, view::ExprView},
};

/// Identifier for a variable, either internal or external.
///
/// Role
/// - `Internal` for bound variables (lambda, forall/exists), `External` for free variables.
/// - Converts to an [`InlineVariable`] (encoded `u32`) for compact storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIs, EnumTryAs)]
pub enum Variable {
    /// An internal variable, e.g., bound in a lambda abstraction or a quantifier.
    Internal(u32),

    /// An external variable, e.g., a constant in a context or environment.
    External(u32),
}

impl Variable {
    /// Maximum valid id for either internal or external variables (31 bits).
    pub const MAX_VARIABLE_ID: u32 = (1 << 31) - 1;

    /// Convert to the compact raw representation used by [`InlineVariable`].
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

    /// Reconstruct a `Variable` from a raw encoded id.
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

/// Identifier for an inline (compact) variable.
///
/// Role
/// - Single-field newtype around `u32` used directly in the encoding format.
/// - Lowest bit distinguishes internal vs external variables; higher bits store the id.
///
/// Display
/// - With default formatting: `a..z` for ids 0..25, otherwise `v<N-26>`.
/// - With alternate formatting (`{:#}`): `A..Z` for ids 0..25 for quick distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineVariable(u32);

impl InlineVariable {
    /// Create from a high-level [`Variable`].
    pub fn new(v: Variable) -> Self {
        Self(v.raw())
    }

    /// Create a new identifier from an encoded numeric id.
    pub fn new_from_raw(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw numeric id.
    pub fn raw(&self) -> u32 {
        self.0
    }

    /// Convert to a high-level [`Variable`], preserving internal/external.
    #[inline]
    pub fn to_variable(&self) -> Variable {
        Variable::new_from_raw(self.0)
    }

    /// Return a short printable symbol for small ids.
    ///
    /// For `variant = false`, the range 0..=25 maps to `a..z`; for `true`, to `A..Z`.
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

/// ======================== Encoding & Traits ========================

impl expr_sealed::Sealed for InlineVariable {}

impl EncodableExpr for InlineVariable {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Variable as u8, Some(self.raw()), &[])
    }
}

impl Expr for InlineVariable {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Variable(*self)
    }
}

impl expr_sealed::Sealed for Variable {}

impl EncodableExpr for Variable {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Variable as u8, Some(self.raw()), &[])
    }
}

impl Expr for Variable {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Variable(InlineVariable::new(*self))
    }
}
