use uuid::Uuid;

use crate::specifications::library::SpecLibrary;

#[derive(Debug, Clone)]
pub struct ModuleSpec {
    pub module_uuid: Uuid,
    pub module_name: Option<String>,
    pub library: SpecLibrary,
}
