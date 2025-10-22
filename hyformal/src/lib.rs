//! Hyformal: zero-copy encodings for a single, unified expression language.
//!
//! This crate now exposes one expression language that includes data-type
//! constructors, program expressions, and logical formulas as first-class
//! expressions. Static well-formedness is intentionally relaxed; validity is
//! expected to be checked at runtime by consumers.
//!
//! Encoding shape
//!  - All encodings use a compact node representation with children first (postfix), then a
//!    single-byte opcode and flags; certain nodes carry a small 32-bit payload.
//!  - Child references are 16-bit offsets into a contiguous buffer, allowing up to 7 children
//!    per node and a maximum buffer size of 64 KiB.
//!
//! Performance
//!  - Dynamic buffers use `smallvec` and keep up to 32 bytes inline before spilling to the heap.
//!  - Borrowed dynamic views never allocate; they borrow the original bytes and decode on the fly.
//!
//! Example
//! ```
//! use hyformal::expr::*;
//! use hyformal::expr::defs::{Bool, ForAll};
//! use hyformal::expr::view::ExprView;
//! use hyformal::expr::variant::ExprType;
//! use hyformal::expr::Expr; // bring trait into scope for view()
//! use hyformal::variable::InlineVariable;
//!
//! // Types as expressions: (Bool -> Bool) x Bool
//! let ty = Bool.lambda(Bool).tuple(Bool);
//! let dyn_ty = ty.encode();
//! assert_eq!(dyn_ty.as_ref().view().type_(), ExprType::Tuple);
//!
//! // Terms and logic as expressions
//! let f = InlineVariable::new_from_raw(0);
//! let x = InlineVariable::new_from_raw(1);
//! let app = f.apply(x);
//! let eq = app.equals(x);
//! let quantified = ForAll { variable: x, dtype: Bool, inner: eq };
//! let dyn_e = quantified.encode();
//! let r = dyn_e.as_ref();
//! let view = r.view();
//! assert!(matches!(view, ExprView::Forall { .. }));
//! ```

/// Mutator for expression rewriting and transformation.
pub mod arena;
/// Encoding internals: compact append-only tree buffer and encoding trait.
pub mod encoding;
/// Expressions API: builders, dispatch, and dynamic encodings.
pub mod expr;
/// Parser for the pretty-printed language.
pub mod parser;
/// Utility types and traits for working with small vectors and slices.
pub mod utils;
/// Inline variables used across types, expressions, and propositions.
pub mod variable;
/// Tree walker for traversing and transforming expressions.
pub mod walker;

pub mod prelude {
    //! Convenient re-exports for end users.
    //!
    //! - `Expr` trait with builder helpers
    //! - Concrete constructors from `defs::*`
    //! - Free-function builders from `func::*`
    //! - Pretty-printing via `PrettyExpr`
    //! - Variable types
    pub use crate::expr::{
        AnyExpr, AnyExprRef, Expr, defs::*, func::*, pretty::PrettyExpr, variant::ExprType,
        view::ExprView,
    };
    pub use crate::variable::{InlineVariable, Variable};

    // Arena helpers
    pub use crate::arena::{ArenaAnyExpr, ExprArenaCtx};

    // Walker ergonomics
    pub use crate::walker::{WalkerHandle, WalkerNodeHandle, walk, walk_no_input};

    // Parser entrypoint
    pub use crate::parser::parse;
}
