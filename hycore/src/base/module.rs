use std::{collections::BTreeMap, sync::Weak};

use hyinstr::modules::{
    InstructionRef, Module,
    operand::{Label, Name},
};
use petgraph::prelude::DiGraphMap;
use uuid::Uuid;

use crate::{
    base::{InstanceContext, ModuleKey},
    theorems::library::TheoremLibrary,
};

/// Contextual information about a [`Function`] within a module.
pub struct FunctionContext {
    /// Unique information about this function.
    pub uuid: Uuid,
    /// The control flow graph of the function.
    pub cfg: DiGraphMap<Label, Name>,
    /// The destination map of the function.
    pub dest_map: BTreeMap<Name, InstructionRef>,
}

/// Aggregates metadata and analysis state for a single module loaded in an instance.
pub struct ModuleContext {
    /// The unique key of this module within the instance.
    pub key: ModuleKey,

    /// Unique identifier for this module. This is consistent across different instances.
    pub uuid: Uuid,

    /// A weak reference to the parent instance context.
    pub instance: Weak<InstanceContext>,

    /// The module itself.
    pub module: Module,

    /// Contexts for internal function analysis.
    pub funcs: BTreeMap<Uuid, FunctionContext>,

    /// Library of properties and specifications (can be used to derive additional
    /// specifications).
    pub library: TheoremLibrary,
}
