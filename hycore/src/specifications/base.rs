//! High-level specifications for analyzing function behavior and associated metadata.
//!
//! This module defines a small specification model used to attach assertions,
//! preconditions, and referenced symbols to a function. The model is intentionally
//! conservative: it provides useful, human-readable approximations for verification
//! and analysis tasks rather than attempting to solve undecidable problems
//! (e.g., general halting).

use std::collections::BTreeSet;

use enum_map::{Enum, EnumMap};
use hyinstr::{
    consts::AnyConst,
    modules::{Function, InstructionRef, operand::Operand, symbol::FunctionPointer},
};
use uuid::Uuid;

/// Approximation of a function's halting behavior under a given condition.
///
/// These variants are intended as coarse, mutually exclusive labels that can be
/// used by analyses and checks. They are not a formal proof of behavior for
/// arbitrary code (the Halting Problem is undecidable), but serve as pragmatic
/// categories for reasoning about termination:
///
/// - `Terminates`: under the linked condition the function is expected to finish
///   in finite time.
/// - `Crashes`: under the linked condition the function is expected to abort
///   abnormally (e.g. due to an unhandled trap or explicit panic).
/// - `Loops`: under the linked condition the function is expected to run forever
///   (non-terminating) without crashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum HaltingBehavior {
    /// The function (under provided condition) is guaranteed to terminate in finite time.
    Terminates,
    /// The function (under provided condition) always crashes (e.g., due to an unhandled exception or illegal operation).
    Crashes,
    /// The function (under provided condition) never terminates and does not crash (e.g., enters an infinite loop).
    Loops,
}

/// A specification attached to a function that collects assertions, assumptions,
/// and references needed for modular verification or analysis.
///
/// A [`Specification`] acts as a meta-function: its [`Specification::function`] field contains the
/// body with embedded meta-assertions and meta-assumptions. The remaining fields
/// are derived helpers that make it convenient to inspect and use those meta
/// elements without repeatedly scanning the function body.
#[derive(Debug, Clone)]
pub struct Specification {
    /// A unique identifier for this specification instance.
    ///
    /// This is used when tracking specifications across different analyses or
    /// transformations to ensure the "proof"s remain associated with the correct
    /// function.
    pub uuid: Uuid,

    /// The function carrying the specification's assertions and assumptions.
    /// Typically this is a thin wrapper or a specially annotated version of the
    /// original function body used for meta-level checks.
    pub function: Function,

    /// A mapping from halting behavior categories to boolean operands that
    /// characterize when each behavior applies.
    ///
    /// Each operand should be a boolean expression (an [`Operand`]) describing the
    /// condition under which the associated [`HaltingBehavior`] is assumed to hold.
    /// Operands are expected to be mutually exclusive where possible, but they do
    /// not need to exhaust all possibilities.
    ///
    /// TODO: Write this as a meta-instruction (something like `invoke_behavior` that takes value in 0..2 with
    /// 0 = terminates, 1 = crashes, 2 = loops) rather than a map.
    pub behavior: EnumMap<HaltingBehavior, Operand>,

    /// Collected references to all assert-style meta-instructions found in `function`.
    ///
    /// These correspond to places where the specification enforces properties that
    /// must hold when the function is executed (postconditions, invariants, etc.).
    _list_asserts: Vec<InstructionRef>,

    /// Collected references to all assume-style meta-instructions found in `function`.
    ///
    /// These are preconditions or environmental assumptions that the analysis or
    /// verifier may take for granted when reasoning about the function.
    _list_assumptions: Vec<InstructionRef>,

    /// All concrete functions referenced by this specification's body (direct calls).
    ///
    /// This set contains only statically known function pointers discovered by
    /// examining the function body. Indirect calls whose target cannot be resolved
    /// to a concrete pointer are intentionally omitted.
    _list_referenced_functions: BTreeSet<FunctionPointer>,
}

impl Specification {
    /// Scan the specification function and update [`Specification::list_asserts`] with all
    /// instructions that represent meta-assertions.
    ///
    /// After calling this method, [`Specification::list_asserts`] will contain references to every
    /// assert-like instruction so callers can iterate them without re-scanning the
    /// function body.
    pub fn derive_meta_asserts(&mut self) {
        self._list_asserts = self
            .function
            .gather_instructions_by_predicate(|instr| instr.is_meta_assert());
    }

    /// Scan the specification function and update [`Specification::list_assumptions`] with all
    /// instructions that represent meta-assumptions (preconditions).
    ///
    /// This isolates precondition-like instructions, simplifying subsequent checks
    /// or transformations that need to treat assumptions specially.
    pub fn derive_meta_assumptions(&mut self) {
        self._list_assumptions = self
            .function
            .gather_instructions_by_predicate(|instr| instr.is_meta_assume());
    }

    /// Scan the function body and populate [`Specification::list_referenced_functions`] with every
    /// directly referenced function pointer.
    ///
    /// Only direct, statically embedded function pointers are collected. Indirect
    /// calls through unknown pointers are not added. This list is useful for
    /// computing call-graphs, linking specification dependencies, and performing
    /// modular analyses that require knowing which external functions a spec
    /// mentions.
    pub fn derive_referenced_functions(&mut self) {
        self._list_referenced_functions = self
            .function
            .iter()
            .filter_map(|(instr, _)| {
                if let Some(call) = instr.try_as_invoke_ref() {
                    use hyinstr::modules::operand::Operand::*;

                    match &call.function {
                        Imm(AnyConst::FuncPtr(func_ptr)) => Some(func_ptr.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect();
    }

    /// Get a reference to the list of meta-assertion instructions.
    pub fn list_asserts(&self) -> &[InstructionRef] {
        &self._list_asserts
    }

    /// Get a reference to the list of meta-assumption instructions.
    pub fn list_assumptions(&self) -> &[InstructionRef] {
        &self._list_assumptions
    }

    /// Get a reference to the set of directly referenced function pointers.
    pub fn list_referenced_functions(&self) -> &BTreeSet<FunctionPointer> {
        &self._list_referenced_functions
    }
}
