use std::sync::{Arc, Weak};

use dashmap::DashMap;
use hyinstr::{
    attached::AttachedFunction,
    modules::{Function, FunctionAnalysis, Module},
};
use parking_lot::RwLock;
use uuid::Uuid;

use crate::{
    base::{InstanceContext, ModuleKey},
    theorems::library::TheoremLibrary,
};

/// Contextual information about a [`Function`] within a module.
pub struct FunctionContext {
    /// Unique information about this function.
    pub uuid: Uuid,
    /// Pointer to the function being analyzed.
    pub function: Arc<Function>,
    /// Function analysis provided by the [`hyinstr`] crate.
    pub analysis: FunctionAnalysis,
    /// A list of attached function specifications.
    pub attached_specs: RwLock<Vec<Arc<RwLock<AttachedFunction>>>>,
}

impl FunctionContext {
    /// Creates a new function context.
    pub fn new(function: Arc<Function>) -> Self {
        let uuid = function.uuid;
        // This cloning only clones the Arc pointer, not the underlying data.
        let analysis = function.clone().analyze();
        Self {
            uuid,
            function,
            analysis,
            attached_specs: RwLock::new(Vec::new()),
        }
    }
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
    /// Key ought to be the function UUID.
    pub funcs: DashMap<Uuid, FunctionContext>,

    /// Library of properties and specifications (can be used to derive additional
    /// specifications).
    pub library: TheoremLibrary,
}
