use std::collections::BTreeMap;

use hyinstr::modules::Module;
use uuid::Uuid;

use crate::axioms::function::FunctionMetadata;

/// Metadata wrapper for a module tying its IR representation to per-function properties.
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    /// Underlying IR module.
    pub module: Module,
    /// Mapping from function UUID to accumulated metadata.
    pub functions_metadata: BTreeMap<Uuid, FunctionMetadata>,
}
