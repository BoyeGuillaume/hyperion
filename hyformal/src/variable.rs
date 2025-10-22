//! Inline variable identifiers used across the crate.
//!
//! Role
//! - Provide compact identifiers for variables used in lambda/quantifiers or external contexts.
//! - Encode to/from expression nodes without allocation.
use strum::{EnumIs, EnumTryAs};

use crate::{
    arena::ArenaAllocableExpr,
    encoding::{
        EncodableExpr,
        tree::{TreeBuf, TreeBufNodeRef},
    },
    expr::{AnyExpr, Expr, variant::ExprType, view::ExprView},
    utils::staticvec::StaticVec,
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

impl From<Variable> for InlineVariable {
    fn from(v: Variable) -> InlineVariable {
        InlineVariable::new(v)
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

    /// Encode a symbolic representation for the variable in hexadecimal form.
    ///
    /// Prefix with '%' for external variables and with '$' for internal variables.
    pub fn to_string(&self) -> StaticVec<u8, 9> {
        let mut buf = StaticVec::new();
        if self.raw() & 1 == 0 {
            buf.push(b'$');
        } else {
            buf.push(b'%');
        }

        for offset in (0..8).rev() {
            let nibble = (self.raw() >> ((offset * 4) + 1)) & 0xF;
            if nibble == 0 && buf.len() == 1 && offset != 0 {
                continue; // Skip leading zeros
            }

            let c = match nibble {
                0..=9 => b'0' + (nibble as u8),
                10..=15 => b'a' + ((nibble as u8) - 10),
                _ => unreachable!(),
            };
            buf.push(c);
        }

        buf
    }

    /// Transfrom any string-like representation back into an `InlineVariable`.
    pub fn from_string<S: AsRef<str>>(s: S) -> Result<Self, &'static str> {
        let s = s.as_ref();
        let mut raw = 0u32;
        let is_internal = match s.chars().next() {
            Some('$') => true,
            Some('%') => false,
            _ => return Err("Invalid variable string: must start with '$' or '%'"),
        };

        for c in s.chars().skip(1) {
            if raw >= (1 << (32 - 5)) {
                return Err("Maximum variable id exceeded, must not exceed 0x7fffffff");
            }

            raw <<= 4;
            let nibble = match c {
                '0'..='9' => c as u32 - '0' as u32,
                'a'..='f' => c as u32 - 'a' as u32 + 10,
                'A'..='F' => c as u32 - 'A' as u32 + 10,
                _ => return Err("Invalid character in variable string"),
            };
            raw |= nibble;
        }

        if is_internal {
            raw = (raw << 1) & !1;
        } else {
            raw = (raw << 1) | 1;
        }

        Ok(InlineVariable::new_from_raw(raw))
    }
}

impl std::fmt::Display for InlineVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buf = self.to_string();
        write!(f, "{}", std::str::from_utf8(&buf).unwrap())
    }
}

// ======================== Encoding & Traits ========================
impl EncodableExpr for InlineVariable {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Variable as u8, Some(self.raw()), &[])
    }
}

impl Expr for InlineVariable {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Variable(*self)
    }
}

impl<'a> ArenaAllocableExpr<'a> for InlineVariable {
    fn alloc_in(
        &self,
        ctx: &'a crate::prelude::ExprArenaCtx<'a>,
    ) -> &'a std::cell::RefCell<crate::prelude::ArenaAnyExpr<'a>> {
        ctx.alloc_expr(crate::prelude::ArenaAnyExpr::ArenaView(
            crate::prelude::ExprView::Variable(*self),
        ))
    }
}

impl EncodableExpr for Variable {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Variable as u8, Some(self.raw()), &[])
    }
}

impl Expr for Variable {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Variable(InlineVariable::new(*self))
    }
}

impl<'a> ArenaAllocableExpr<'a> for Variable {
    fn alloc_in(
        &self,
        ctx: &'a crate::prelude::ExprArenaCtx<'a>,
    ) -> &'a std::cell::RefCell<crate::prelude::ArenaAnyExpr<'a>> {
        ctx.alloc_expr(crate::prelude::ArenaAnyExpr::ArenaView(
            crate::prelude::ExprView::Variable(InlineVariable::new(*self)),
        ))
    }
}
