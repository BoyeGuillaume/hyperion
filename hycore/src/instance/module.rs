use hyinstr::modules::Module;
use uuid::Uuid;

pub struct ModuleContext {
    /// Unique information about this module.
    pub uuid: Uuid,

    /// The module itself.
    pub module: Module,
}
