use std::u32;

use hyinstr::modules::{
    Instruction,
    instructions::HyInstr,
    misc::Assert,
    operand::{Label, MetaName},
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
    // Sorted by `FunctionPoint`
    internals: Vec<(FunctionPoint, HyInstr)>,
}

impl FunctionAxioms {
    // fn locate_dest(&self, name: MetaName) -> Result<usize, usize> {
    //     // Binary search for the destination name
    //     self.internals
    //         .binary_search_by_key(&name.0, |x| x.destination().unwrap_or(u32::MAX))
    // }

    // /// Insert in internals, maintaining sorted order by destination name.
    // pub fn insert_internals(&mut self, instr: HyInstr) {
    //     debug_assert!(
    //         self.check_condition(),
    //         "FunctionPropertiesMap internals are not sorted by destination name"
    //     );

    //     // Destination should be unique in postconditions
    //     let dest = instr
    //         .destination()
    //         .expect("Only instructions with destinations can be inserted in internals, `Assert` should be in pre/postconditions");
    //     let index = self.locate_dest(MetaName(dest));
    //     self.internals.insert(index.unwrap_or_else(|i| i), instr);
    // }

    // /// Get internal instruction by its MetaName destination.
    // pub fn get_internals(&self, name: MetaName) -> Option<&HyInstr> {
    //     debug_assert!(
    //         self.check_condition(),
    //         "FunctionPropertiesMap internals are not sorted by destination name"
    //     );

    //     match self.locate_dest(name) {
    //         Err(_) => None,
    //         Ok(index) => Some(&self.internals[index]),
    //     }
    // }
}

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// A list of function properties maps associated with this function.
    pub properties: Vec<FunctionAxioms>,
}
