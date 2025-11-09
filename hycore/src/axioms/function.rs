use std::collections::BTreeMap;

use hyinstr::modules::{
    Instruction,
    instructions::HyInstr,
    misc::{Assert, Assume},
    operand::{Label, MetaName, Operand},
};
use smallvec::SmallVec;
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
    pub postconditions: Vec<Assert>,
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
    intermediary: Vec<AxiomIntermediaryEntry>,
    asserts: Vec<InternalAssert>,
    assumptions: Vec<Assume>, // Should be `attached` to a free-variable;

    /// Indexes for iteration by point (`block_map`) and direct lookup by meta destination (`dest_index_map`).
    block_map: Vec<(FunctionPoint, usize)>,
    dest_index_map: BTreeMap<MetaName, usize>,
    assumptions_freevar_map: BTreeMap<MetaName, Vec<Assume>>,

    /// Next free destination allocator (monotonic).
    next_meta_name: u32,
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
    pub assert: Assert,
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

impl FunctionAxioms {
    /// Insert a new internal instruction at a given program point.
    ///
    /// Performs the following:
    /// 1. Binary-search inserts point/destination indexes maintaining sort order.
    /// 2. Enforces destination uniqueness (panics on conflict).
    /// 3. Registers free variable destinations for future assumption attachment.
    ///
    /// Panics if the instruction lacks a destination or duplicates an existing one.
    pub fn add_intermediary(&mut self, point: FunctionPoint, instr: HyInstr) {
        // Insert sorted in block_map
        let block_index = {
            let index = self
                .block_map
                .binary_search_by_key(&point, |(p, _)| *p)
                .unwrap_or_else(|i| i);
            for i in index..self.block_map.len() {
                if self.block_map[i].0 != point {
                    break;
                }
            }
            index
        };

        self.block_map
            .insert(block_index, (point, self.intermediary.len()));

        // Insert sorted in dest_index_map
        let dest = instr.destination().expect("Internal instructions must have a destination. Assertions should be stored separately.");
        self.next_meta_name = std::cmp::max(self.next_meta_name, dest + 1);

        // Assert does not already exist
        if self
            .dest_index_map
            .insert(MetaName(dest), self.intermediary.len())
            .is_some()
        {
            panic!(
                "Internal instruction with destination {:?} already exists.",
                dest
            );
        }

        // If instr is a free variable, add to freevar map
        if let Some(freevar) = instr.try_as_free_var_ref() {
            self.assumptions_freevar_map
                .insert(MetaName(freevar.dest), Vec::new());
        }

        // Add to internals
        self.intermediary.push(AxiomIntermediaryEntry {
            point: point,
            instr: instr,
        });
    }

    /// Insert an assertion at the specified program point unless it already exists.
    /// Duplicate assertions (same point and condition) are ignored quietly.
    pub fn assert(&mut self, point: FunctionPoint, condition: Operand) {
        // Unsure that the assertion does not already exist
        let internal_assert = InternalAssert {
            point,
            assert: Assert {
                condition: condition.clone(),
            },
        };

        if self
            .asserts
            .iter()
            .find(|x| *x == &internal_assert)
            .is_some()
        {
            return;
        }

        self.asserts.push(internal_assert);
    }

    /// Insert an assumption and attach it to all reachable free variables.
    ///
    /// Traverses the operand dependency graph backward from the assumption's
    /// root condition through intermediary instructions until free variables are
    /// discovered. Panics if no free variable is reachable (likely a modeling error).
    pub fn assume(&mut self, assumption: Assume) {
        // Iterate over all dependencies of the assumption (direct as well as indirect) until we either reach
        // program arguments or a free variable(s)
        let mut stack: SmallVec<[_; 6]> = SmallVec::new();
        let mut to_be_added: SmallVec<[_; 6]> = SmallVec::new();
        stack.push(&assumption.condition);

        // Loop until stack is empty
        while let Some(operand) = stack.pop() {
            match operand {
                Operand::Meta(meta_name) => {
                    // free variable only exists in meta-space so we can only attach assumptions to free variables
                    if let Some(entry) = self.get(*meta_name) {
                        // If the instruction is a free variable, attach the assumption
                        if let Some(freevar) = entry.instr.try_as_free_var_ref() {
                            to_be_added.push((MetaName(freevar.dest), assumption.clone()));
                        } else {
                            // Otherwise, push its operands to the stack
                            for op in entry.instr.operands() {
                                stack.push(op);
                            }
                        }
                    } else {
                        panic!(
                            "MetaName {:?} not found in FunctionAxioms intermediary instructions.",
                            meta_name
                        );
                    }
                }
                _ => {}
            }
        }
        drop(stack);

        // To_be_added now contains all free variables to which we need to attach the assumption
        if to_be_added.is_empty() {
            panic!(
                "No free variable found in direct/indirect dependencies of assumption {:?}.",
                assumption
            );
        }
        for (dest, assumption) in to_be_added {
            self.assumptions_freevar_map
                .entry(dest)
                .or_default()
                .push(assumption.clone());
        }

        // Finally add the assumptio
        self.assumptions.push(assumption);
    }

    /// Allocate and return the next fresh destination meta name.
    pub fn next_meta_name(&mut self) -> MetaName {
        let name = MetaName(self.next_meta_name);
        self.next_meta_name += 1;
        name
    }

    /// Iterate over all unique assertions (unordered).
    pub fn iter_asserts(&self) -> impl Iterator<Item = &InternalAssert> {
        self.asserts.iter()
    }

    /// Iterate over all intermediary instructions (append order).
    pub fn iter_internals(&self) -> impl Iterator<Item = &AxiomIntermediaryEntry> {
        self.intermediary.iter()
    }

    /// Lookup an intermediary instruction by its destination meta name.
    pub fn get(&self, name: MetaName) -> Option<&AxiomIntermediaryEntry> {
        match self.dest_index_map.get(&name) {
            Some(&index) => Some(&self.intermediary[index]),
            None => None,
        }
    }

    /// Iterate over all intermediary instructions located at `point`.
    /// Efficient: uses binary search over the point index.
    pub fn iter_internals_at(&self, point: FunctionPoint) -> impl Iterator<Item = &HyInstr> {
        let start_point = match self.block_map.binary_search_by_key(&point, |(p, _)| *p) {
            Ok(mut index) => {
                while index > 0 && self.block_map[index - 1].0 == point {
                    index -= 1;
                }
                Some(index)
            }
            Err(_) => None,
        };

        struct It<'a> {
            axioms: &'a FunctionAxioms,
            current_index: usize,
            point: FunctionPoint,
        }

        impl<'a> Iterator for It<'a> {
            type Item = &'a HyInstr;

            fn next(&mut self) -> Option<Self::Item> {
                if self.current_index >= self.axioms.block_map.len() {
                    return None;
                }
                let (current_point, instr_index) = &self.axioms.block_map[self.current_index];
                if *current_point != self.point {
                    return None;
                }
                self.current_index += 1;
                Some(&self.axioms.intermediary[*instr_index].instr)
            }
        }

        It {
            axioms: self,
            current_index: start_point.unwrap_or(self.block_map.len()),
            point,
        }
    }

    /// Iterate over all assertions located at `point`.
    pub fn iter_asserts_at(&self, point: FunctionPoint) -> impl Iterator<Item = &Assert> {
        self.asserts
            .iter()
            .filter(move |a| a.point == point)
            .map(|a| &a.assert)
    }

    /// Iterate over all of the assumptions attached to a given free variable.
    pub fn iter_assumptions_for_freevar(&self, freevar: MetaName) -> impl Iterator<Item = &Assume> {
        // If panic, freevar not found
        self.assumptions_freevar_map.get(&freevar).unwrap().iter()
    }
}

/// Bundle of all per-function property sets.
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Collection of disjoint property sets (e.g., produced by different derivators or phases).
    pub properties: Vec<FunctionAxioms>,
}
