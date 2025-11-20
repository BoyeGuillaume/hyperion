use std::collections::BTreeMap;

use hyinstr::modules::{
    instructions::HyInstr,
    meta::MetaAssert,
    operand::{Label, MetaLabel, Operand},
};
use strum::{EnumIs, EnumTryAs};

/// Semantic behavior of a function with respect to halting and failure modes.
///
/// A function is classified (possibly under a predicate – see [`BehaviorCase`]) into
/// one of several categories capturing termination or abnormal execution.
/// "Unknown" categories encode epistemic uncertainty rather than nondeterminism.
///
/// The lattice (ordering by information) is roughly:
///
/// ```text
///            Unknown
///        /      |     \
///    MayLoop  MayCrash  (both)
///      |         |        \
///   Looping   Crashes    Halting
/// ```
///
/// (Diagram is informal; Halting / Looping / Crashes are incomparable base cases.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionBehavior {
    /// Halting (guaranteed to complete in finite time under the associated guard).
    Halting,

    /// Non-halting (diverges; never completes under the associated guard).
    Looping,

    /// Crashes (provably aborts execution under the associated guard).
    Crashes,

    /// Unknown behavior (could be halting or looping; crash excluded).
    MayLoop,

    /// Unknown behavior (could be halting or crashing; divergence excluded).
    MayCrash,

    /// Fully unknown (no information: could halt, loop, or crash).
    Unknown,
}

impl FunctionBehavior {
    /// Returns true iff this classification does not exclude the possibility of a crash.
    pub fn may_crash(&self) -> bool {
        matches!(
            self,
            FunctionBehavior::Crashes | FunctionBehavior::MayCrash | FunctionBehavior::Unknown
        )
    }

    /// Returns true iff this classification does not exclude divergence.
    pub fn may_loop(&self) -> bool {
        matches!(
            self,
            FunctionBehavior::Looping | FunctionBehavior::MayLoop | FunctionBehavior::Unknown
        )
    }
}

/// Identifies a logical program point or abstract state within a function.
///
/// Program points are used to anchor assertions, intermediary instructions,
/// and behavior cases. They form a partially ordered control-flow abstraction:
/// * `Entry` precedes all internal labels.
/// * `Internal(Label)` refer to labelled points (implementation-defined ordering).
/// * `Exit` succeeds all internal labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumIs, EnumTryAs)]
pub enum FunctionPoint {
    /// The entry point of a function (preconditions / assumptions typically attached here).
    Entry,
    /// Internal labelled point within a function body.
    Internal(Label),
    /// The exit point of a function (postconditions / guarantees attached here).
    Exit,
}

/// A collection of postconditions that are jointly sufficient to prove semantic
/// equivalence between two implementations of a function under the same pre-state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SufficientEquivalentPostcondition {
    pub postconditions: Vec<MetaAssert>,
}

/// Aggregated logical properties and intermediary definitions for a single function.
///
/// This structure accumulates logical facts in an append-only manner. Public
/// fields (like [`FunctionAxioms::behaviors`]) can be extended, while internal indexing fields
/// maintain fast lookups for program points and meta names.
///
/// Violations panic early to surface logic errors during derivation.
#[derive(Debug, Clone, Default)]
pub struct FunctionAxioms {
    pub behaviors: Vec<BehaviorCase>,

    /// Raw internal instructions in append order. DO NOT reorder externally; indexes are shared.
    pub intermediary: Vec<AxiomIntermediaryEntry>,
    pub asserts: Vec<InternalAssert>,

    /// Indexes for iteration by point (`block_map`) and direct lookup by meta destination (`dest_index_map`).
    pub block_map: Vec<(FunctionPoint, usize)>,
    pub dest_index_map: BTreeMap<MetaLabel, usize>,

    /// Next free destination allocator (monotonic).
    pub next_meta_name: u32,
}

/// Internal intermediary instruction entry associated with a program point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxiomIntermediaryEntry {
    pub point: FunctionPoint,
    pub instr: HyInstr,
}

/// Internal assertion tied to a program point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalAssert {
    pub point: FunctionPoint,
    pub assert: MetaAssert,
}

/// A single guarded behavior classification of a function.
///
/// When `guard` holds at the entry point, the function is guaranteed to match
/// the `behavior` classification. Multiple cases can overlap however strict
/// ordering according to the lattice in [`FunctionBehavior`] is enforced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorCase {
    /// Predicate over the entry state under which `behavior` holds
    pub guard: Operand,
    /// Classified behavior under the associated guard.
    pub behavior: FunctionBehavior,
}

/// Bundle of all per-function property sets.
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Collection of disjoint property sets (e.g., produced by different derivators or phases).
    pub properties: Vec<FunctionAxioms>,
}
