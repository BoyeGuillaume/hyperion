//! Hyformal: zero-copy encodings for simple types, expressions, and propositions.
//!
//! This crate provides a small set of building blocks for representing:
//! - data types (via [`dtype`])
//! - expressions (via [`expr`])
//! - logical propositions (via [`prop`])
//!
//! The design centers around two complementary forms for each domain:
//! - a typed, composable builder API (e.g., `TBool`, `App`, `And`, …) that implements
//!   lightweight encoding to a compact byte buffer; and
//! - a zero-copy, dynamically-typed view (`Dyn*` and `DynBorrowed*`) that can decode the
//!   previously encoded bytes without allocating or cloning.
//!
//! These forms make it easy to build structures in a type-safe way and then transmit,
//! store, or analyze them efficiently with minimal allocation. Decoding yields a
//! dispatcher enum (e.g., [`dtype::dispatch::DTypeDispatch`]) describing the top-level
//! shape and providing direct access to children.
//!
//! Encoding shape
//!  - All encodings use Reverse Polish Notation (postfix): children first, then an opcode
//!    byte, with lengths for some right operands. See [`encoding::magic`].
//!  - Lengths are compact u64 varints; see [`encoding::integer`].
//!
//! Performance
//!  - `Dyn*` buffers use `smallvec` and keep up to 32 bytes inline before spilling to the heap.
//!  - `DynBorrowed*` variants never allocate; they borrow the original bytes and decode on the fly.
//!
//! Examples
//! ```
//! use hyformal::{dtype::*, expr::*, prop::*, variable::InlineVariable};
//! use hyformal::dtype::dispatch::DTypeDispatch;
//!
//! // Types: (Bool -> Bool) x Bool
//! let ty = TBool.app(TBool).tuple(TBool);
//! let dyn_ty = ty.encode();
//! assert!(matches!(dyn_ty.decode_dtype_concrete(), DTypeDispatch::Tuple(_, _)));
//!
//! // Expressions and propositions
//! let f = InlineVariable::new(0); // a function variable
//! let x = InlineVariable::new(1); // an argument variable
//! let app = f.apply(x); // f(x)
//! let eq = app.equals(x); // f(x) == x
//! let quantified = ForAll { variable: x, dtype: TBool, inner: eq };
//! let dyn_prop = hyformal::prop::Prop::encode(&quantified);
//! assert!(dyn_prop.decode_prop_borrowed().is_for_all());
//! ```
#![deny(missing_docs)]

/// Types API: constructors, dispatch, and dynamic encodings.
pub mod dtype;
pub(crate) mod encoding;
#[macro_use]
/// Operator sugar macros used by propositions (BitAnd, BitOr, Not).
pub mod ops;
/// Expressions API: builders, dispatch, and dynamic encodings.
pub mod expr;
/// Propositions API: builders, dispatch, and dynamic encodings.
pub mod prop;
/// Inline variables used across types, expressions, and propositions.
pub mod variable;
