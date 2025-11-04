use std::collections::BTreeMap;

use hyinstr::modules::Module;
use uuid::Uuid;

use crate::attributes::function::FunctionMetadata;

#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub module: Module,
    pub functions_metadata: BTreeMap<Uuid, FunctionMetadata>,
}
