use semver::Version;

use crate::{base::module::ModuleContext, utils::error::HyResult};

pub mod api;
pub mod ext;
pub mod meta;
pub mod module;

build_info::build_info!(fn retrieve_build_info);

/// Internal instance context.
pub struct InstanceContext {
    /// Version of the instance context.
    pub version: Version,

    /// Information about the application that created this instance.
    pub application_name: String,
    pub application_version: Version,
    pub engine_version: Version,
    pub engine_name: Option<String>,

    /// A list of modules loaded into this instance
    pub modules: Vec<ModuleContext>,
}

impl InstanceContext {
    pub fn create(instance_create_info: &api::InstanceCreateInfo) -> HyResult<Self> {
        // Construct state about the application.
        let application_name = instance_create_info
            .application_info
            .application_name
            .to_string();
        let application_version = instance_create_info
            .application_info
            .application_version
            .into();
        let engine_version = instance_create_info.application_info.engine_version.into();
        let engine_name = instance_create_info
            .application_info
            .engine_name
            .map(|name| name.to_string());

        // Retrieve build info for the current crate.
        let _build_info = retrieve_build_info();

        Ok(InstanceContext {
            version: _build_info.crate_info.version.clone(),
            application_name,
            application_version,
            engine_name,
            engine_version,
            modules: Vec::new(),
        })
    }
}
