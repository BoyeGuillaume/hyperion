//! Inline variable identifiers used across the crate.
//!
//! A compact newtype around `u64` with helpers for display and application.
use strum::{EnumIs, EnumTryAs};

use crate::{
    encoding::{
        LegacyRawEncodable,
        integer::{encode_u64, encoded_size_u64},
    },
    expr::{Expr, defs::App},
};

/// Identifier for a variable, either internal or external.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIs, EnumTryAs)]
pub enum Variable {
    /// An internal variable, e.g., bound in a lambda abstraction or a quantifier.
    Internal(u64),

    /// An external variable, e.g., a constant in a context or environment.
    External(u64),
}

impl Variable {
    /// Maximum valid id for either internal or external variables (63 bits).
    pub const MAX_VARIABLE_ID: u64 = (1 << 63) - 1;

    /// Create a new internal variable with the given numeric id.
    pub fn raw(&self) -> u64 {
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
    pub fn new_from_raw(id: u64) -> Self {
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
/// external constants (e.g., in a context or environment). The LSB of the `u64` is reserved to
/// distinguish these two cases;
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineVariable(u64);

impl InlineVariable {
    /// Create from a variable
    pub fn new(v: Variable) -> Self {
        Self(v.raw())
    }

    /// Create a new identifier with the given numeric id.
    pub fn new_from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw numeric id.
    pub fn raw(&self) -> u64 {
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

    #[inline]
    /// Apply this variable as a function to an argument, producing an application expression.
    pub fn apply<A: Expr>(self, arg: A) -> App<A> {
        App { func: self, arg }
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

impl LegacyRawEncodable for InlineVariable {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let size = encode_u64(self.raw(), f);
        f(&[crate::encoding::legacy_magic::MISC_VAR]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        encoded_size_u64(self.raw()) + 1
    }
}

impl Into<Variable> for InlineVariable {
    fn into(self) -> Variable {
        self.to_variable()
    }
}
