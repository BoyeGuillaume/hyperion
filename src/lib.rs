//! Hyformal: zero-copy encodings for a single, unified expression language.
//!
//! This crate now exposes one expression language that includes data-type
//! constructors, program expressions, and logical formulas as first-class
//! expressions. Static well-formedness is intentionally relaxed; validity is
//! expected to be checked at runtime by consumers.
//!
//! Encoding shape
//!  - All encodings use Reverse Polish Notation (postfix): children first, then an opcode
//!    byte, with lengths for some right operands. See [`encoding::magic`].
//!  - Lengths are compact u64 varints; see [`encoding::integer`].
//!
//! Performance
//!  - Dynamic buffers use `smallvec` and keep up to 32 bytes inline before spilling to the heap.
//!  - Borrowed dynamic views never allocate; they borrow the original bytes and decode on the fly.
//!
//! Example
//! ```
//! use hyformal::expr::*;
//! use hyformal::expr::defs::{Bool, ForAll};
//! use hyformal::expr::view::ExprDispatchVariant;
//! use hyformal::variable::InlineVariable;
//! use strum::IntoDiscriminant;
//!
//! // Types as expressions: (Bool -> Bool) x Bool
//! let ty = Bool.func(Bool).tuple(Bool);
//! let dyn_ty = ty.encode();
//! assert!(matches!(dyn_ty.decode_expr_borrowed().discriminant(), ExprDispatchVariant::Tuple));
//!
//! // Terms and logic as expressions
//! let f = InlineVariable::new_from_raw(0);
//! let x = InlineVariable::new_from_raw(1);
//! let app = f.apply(x);
//! let eq = app.equals(x);
//! let quantified = ForAll { variable: x, dtype: Bool, inner: eq };
//! let dyn_e = quantified.encode();
//! let view = dyn_e.decode_expr_borrowed();
//! assert!(view.is_for_all());
//! ```

pub mod encoding;
/// Expressions API: builders, dispatch, and dynamic encodings.
pub mod expr;
/// Parser for the pretty-printed language.
pub mod parser;
pub mod utils;
/// Inline variables used across types, expressions, and propositions.
pub mod variable;

pub mod prelude {
    pub use crate::expr::{Expr, defs::*, func::*, pretty::PrettyExpr};
    pub use crate::variable::{InlineVariable, Variable};
}
