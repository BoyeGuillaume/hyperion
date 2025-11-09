use std::collections::BTreeMap;

use hyinstr::modules::{
    Instruction,
    instructions::HyInstr,
    misc::{Assert, Assume},
    operand::{Label, MetaName, Operand},
};
use smallvec::SmallVec;
use strum::{EnumIs, EnumTryAs};

/// Behavior of a function with respect to halting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionBehavior {
    /// Halting (complete in finite time)
    Halting,

    /// Non-halting (never completes for all inputs matching associated conditions)
    Looping,

    /// Crashes (aborts execution for all inputs matching associated conditions)
    Crashes,

    /// Unknown behavior (could be halting, looping)
    MayLoop,

    /// Unknown behavior (could be halting, crashing)
    MayCrash,

    /// Unknown behavior (could be looping, crashing, or halting)
    Unknown,
}

impl FunctionBehavior {
    /// Returns true if the function may crash.
    pub fn may_crash(&self) -> bool {
        matches!(
            self,
            FunctionBehavior::Crashes | FunctionBehavior::MayCrash | FunctionBehavior::Unknown
        )
    }

    /// Returns true if the function may loop
    pub fn may_loop(&self) -> bool {
        matches!(
            self,
            FunctionBehavior::Looping | FunctionBehavior::MayLoop | FunctionBehavior::Unknown
        )
    }
}

/// Identifies a logical program point or abstract state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumIs, EnumTryAs)]
pub enum FunctionPoint {
    /// The entry point of a function (preconditions are checked here)
    Entry,
    /// Internal point within a function (labeled by a `Label`)
    Internal(Label),
    /// The exit point of a function (postconditions are checked here)
    Exit,
}

/// A collection of postconditions that are sufficient to prove equivalence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SufficientEquivalentPostcondition {
    pub postconditions: Vec<Assert>,
}

/// A collection of properties associated with a function.
#[derive(Debug, Clone, Default)]
pub struct FunctionAxioms {
    pub behaviors: Vec<BehaviorCase>,

    /// Unsorted, SHOULD NOT MODIFY, append only due to indexing that needs to be preserved
    intermediary: Vec<AxiomIntermediaryEntry>,
    asserts: Vec<InternalAssert>,
    assumptions: Vec<Assume>, // Should be `attached` to a free-variable;

    /// Indexes to find IntermediaryInstr by FunctionPoint (iterator) and by MetaName (direct)
    block_map: Vec<(FunctionPoint, usize)>,
    dest_index_map: BTreeMap<MetaName, usize>,
    assumptions_freevar_map: BTreeMap<MetaName, Vec<Assume>>,

    /// Next free MetaName index for internal instructions
    next_meta_name: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxiomIntermediaryEntry {
    pub point: FunctionPoint,
    pub instr: HyInstr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalAssert {
    pub point: FunctionPoint,
    pub assert: Assert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorCase {
    /// Predicate over the entry state under which `behavior` holds
    pub guard: Operand,
    pub behavior: FunctionBehavior,
}

impl FunctionAxioms {
    /// Insert in internals
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

    /// Insert an assertion
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

    /// Insert an assumption
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

    /// Get next free MetaName for internal instructions
    pub fn next_meta_name(&mut self) -> MetaName {
        let name = MetaName(self.next_meta_name);
        self.next_meta_name += 1;
        name
    }

    /// Iterate over all assertions.
    pub fn iter_asserts(&self) -> impl Iterator<Item = &InternalAssert> {
        self.asserts.iter()
    }

    /// Iterate over all intermediary instructions.
    pub fn iter_internals(&self) -> impl Iterator<Item = &AxiomIntermediaryEntry> {
        self.intermediary.iter()
    }

    /// Find instruction by its destination MetaName.
    pub fn get(&self, name: MetaName) -> Option<&AxiomIntermediaryEntry> {
        match self.dest_index_map.get(&name) {
            Some(&index) => Some(&self.intermediary[index]),
            None => None,
        }
    }

    /// Iterate over all instructions at a given FunctionPoint.
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

    /// Iterate over all assertions at a given FunctionPoint.
    pub fn iter_asserts_at(&self, point: FunctionPoint) -> impl Iterator<Item = &Assert> {
        self.asserts
            .iter()
            .filter(move |a| a.point == point)
            .map(|a| &a.assert)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// A list of function properties maps associated with this function.
    pub properties: Vec<FunctionAxioms>,
}
