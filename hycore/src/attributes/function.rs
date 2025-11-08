use std::collections::BTreeMap;

use hyinstr::modules::{
    Instruction,
    instructions::HyInstr,
    misc::Assert,
    operand::{Label, MetaName, Operand},
};
use strum::{EnumIs, EnumTryAs};

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
pub struct SufficientEquivalencePostcondition {
    pub postconditions: Vec<Assert>,
}

/// A collection of properties associated with a function.
#[derive(Debug, Clone, Default)]
pub struct FunctionAxioms {
    /// Unsorted, SHOULD NOT MODIFY, append only due to indexing that needs to be preserved
    intermediary: Vec<AxiomIntermediaryEntry>,
    asserts: Vec<InternalAssert>,

    /// Indexes to find IntermediaryInstr by FunctionPoint (iterator) and by MetaName (direct)
    block_map: Vec<(FunctionPoint, usize)>,
    dest_index_map: BTreeMap<MetaName, usize>,

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
    pub fn iter_internals_at_point(&self, point: FunctionPoint) -> impl Iterator<Item = &HyInstr> {
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
}

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// A list of function properties maps associated with this function.
    pub properties: Vec<FunctionAxioms>,
}
