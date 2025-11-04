use std::u32;

use hyinstr::modules::{Instruction, instructions::HyInstr, misc::Assert, operand::MetaName};

/// A collection of postconditions that are sufficient to prove equivalence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SufficientEquivalencePostcondition {
    pub postconditions: Vec<Assert>,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionProperties {
    pub internals: Vec<HyInstr>,

    /// Preconditions associated with this function property.
    pub preconditions: Vec<Assert>,

    /// List of intermediate assertions associated with this function property.
    /// TODO: Implement `phi` and figure out how to represent them.
    pub assertions: Vec<Assert>,

    /// List of postconditions associated with this function property. Condition should
    /// hold at the end of the function.
    pub postconditions: Vec<Assert>,

    /// Sufficient equivalence postconditions associated with this function property.
    /// All element of this field should also be present in `postconditions`.
    pub sufficient_equivalence_postconditions: Vec<SufficientEquivalencePostcondition>,
}

impl FunctionProperties {
    fn check_condition(&self) -> bool {
        self.internals
            .iter()
            .is_sorted_by_key(|x| x.destination().unwrap())
    }

    fn locate_dest(&self, name: MetaName) -> Result<usize, usize> {
        // Binary search for the destination name
        self.internals
            .binary_search_by_key(&name.0, |x| x.destination().unwrap_or(u32::MAX))
    }

    /// Insert in internals, maintaining sorted order by destination name.
    pub fn insert_internals(&mut self, instr: HyInstr) {
        debug_assert!(
            self.check_condition(),
            "FunctionPropertiesMap internals are not sorted by destination name"
        );

        // Destination should be unique in postconditions
        let dest = instr
            .destination()
            .expect("Only instructions with destinations can be inserted in internals, `Assert` should be in pre/postconditions");
        let index = self.locate_dest(MetaName(dest));
        self.internals.insert(index.unwrap_or_else(|i| i), instr);
    }

    /// Get internal instruction by its MetaName destination.
    pub fn get_internals(&self, name: MetaName) -> Option<&HyInstr> {
        debug_assert!(
            self.check_condition(),
            "FunctionPropertiesMap internals are not sorted by destination name"
        );

        match self.locate_dest(name) {
            Err(_) => None,
            Ok(index) => Some(&self.internals[index]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// A list of function properties maps associated with this function.
    pub properties: Vec<FunctionProperties>,
}
