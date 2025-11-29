use uuid::Uuid;

use crate::specifications::library::SpecLibrary;

#[derive(Debug, Clone)]
pub struct ModuleSpec {
    /// The UUID of the module.
    pub module_uuid: Uuid,

    /// The name of the module (if any).
    pub module_name: Option<String>,

    /// The library specification associated with the module.
    pub library: SpecLibrary,
}
