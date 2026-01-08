use std::collections::BTreeMap;

use hyinstr::modules::{
    InstructionRef, Module,
    operand::{Label, Name},
};
use petgraph::prelude::DiGraphMap;
use uuid::Uuid;

use crate::specifications::library::SpecificationLibrary;

/// Contextual information about a [`Function`] within a module.
pub struct FunctionContext {
    /// Unique information about this function.
    pub uuid: Uuid,
    /// The control flow graph of the function.
    pub cfg: DiGraphMap<Label, Name>,
    /// The destination map of the function.
    pub dest_map: BTreeMap<Name, InstructionRef>,
}

pub struct ModuleContext {
    /// Unique information about this module.
    pub uuid: Uuid,

    /// The module itself.
    pub module: Module,

    /// Contexts for internal function analysis.
    pub funcs: BTreeMap<Uuid, FunctionContext>,

    /// Library of properties and specifications (can be used to derive additional
    /// specifications).
    pub library: SpecificationLibrary,
}
