//! Inline variable identifiers used across the crate.
//!
//! A compact newtype around `u64` with helpers for display and application.
use crate::{
    encoding::RawEncodable,
    expr::{Expr, defs::App},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Identifier for an inline variable.
pub struct InlineVariable(u64);

impl InlineVariable {
    /// Create a new identifier with the given numeric id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw numeric id.
    pub fn id(&self) -> u64 {
        self.0
    }

    /// Return a short printable symbol for small ids.
    ///
    /// For `variant = false`, the range 0..=25 maps to `A..Z`; for `true`, to `a..z`.
    /// Larger ids return `None`.
    pub fn symbol(&self, variant: bool) -> Option<char> {
        let base = if variant { b'a' } else { b'A' };
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
            write!(f, "v{}", self.0)
        }
    }
}

impl RawEncodable for InlineVariable {
    fn encode_raw(&self, buf: &mut crate::encoding::DynBuf) {
        crate::encoding::integer::encode_u64(self.id(), buf);
        buf.push(crate::encoding::magic::VAR_INLINE);
    }
}
