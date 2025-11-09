//! # hycore: Core property model and derivation runtime
//!
//! `hycore` provides the core data model and derivation engine used by Hyperion
//! to express, attach, and compute semantic properties of IR programs.
//!
//! At a high level, the crate offers two complementary pieces:
//!
//! - The [`attributes`] module defines immutable, structured metadata you can attach to
//!   modules and functions: assertions, assumptions, intermediate logical definitions,
//!   and behavior cases (halting, looping, crashing) guarded by predicates.
//! - The [`derivator`] module defines a small runtime and traits to implement iterative
//!   property derivation passes (aka "derivators"). Derivators can step under budgets
//!   (time/iterations), be paused/resumed with a serializable context, and finalize
//!   their results on termination.
//!
//! The property language is intentionally minimal and piggybacks on the `hyinstr` crate
//! for operands, instructions, and analysis context. This keeps the surface area small
//! while allowing rich logical expressions and SSA-like internal definitions.
//!
//! ## Quick start
//!
//! Add or derive properties for a function by creating a [`FunctionAxioms`](attributes::function::FunctionAxioms)
//! instance and populating it with assertions, assumptions, and internal definitions.
//!
//! ```rust
//! use hycore::axioms::function::{FunctionAxioms, FunctionPoint, FunctionBehavior, BehaviorCase};
//! use hyinstr::modules::operand::Operand;
//!
//! // Initialize a new property set for a function.
//! let mut props = FunctionAxioms::default();
//!
//! // Allocate a fresh meta operand to use as a symbolic guard.
//! let guard_meta = Operand::Meta(props.next_meta_name());
//!
//! // Add an assertion about the symbolic guard at function exit.
//! props.assert(FunctionPoint::Exit, guard_meta.clone());
//!
//! // Record a halting behavior case guarded by the same symbolic predicate.
//! props.behaviors.push(BehaviorCase {
//!     guard: guard_meta,
//!     behavior: FunctionBehavior::Halting,
//! });
//! ```
//!
//! To compute properties automatically, implement [`derivator::PropDerivator`] and call
//! [`derivator::PropDerivator::derive_props`]. The runtime takes care of budgeting and
//! optional finalization.
//!
//! ## Design principles
//!
//! - Properties are modeled as pure data structures that can be accumulated and inspected.
//! - Derivation is explicit, iterative, and interruptible; no global mutable state.
//! - Integration with `hyinstr` ensures property expressions map directly to IR-level
//!   symbols (operands, labels, meta names) for precise reasoning.
//!
//! See module docs for details and full examples.
pub mod axioms;
pub mod derivator;
