use semver::Version;

use crate::{instance::module::ModuleContext, utils::error::HyResult};

pub mod api;
pub mod module;

build_info::build_info!(fn retrieve_build_info);

pub struct InstanceContext {
    /// Version of the instance context.
    pub(super) version: Version,

    /// Information about the application that created this instance.
    pub(super) application_name: String,
    pub(super) application_version: Version,

    /// A list of modules loaded into this instance
    pub modules: Vec<ModuleContext>,
}

impl InstanceContext {
    pub fn create_instance(instance_create_info: &api::InstanceCreateInfo) -> HyResult<Self> {
        // Construct state about the application.
        let application_name = instance_create_info
            .application_info
            .application_name
            .to_string();
        let application_version = instance_create_info.application_info.version.into();

        // Retrieve build info for the current crate.
        let _build_info = retrieve_build_info();

        Ok(InstanceContext {
            version: _build_info.crate_info.version.clone(),
            application_name,
            application_version,
            modules: Vec::new(),
        })
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn application_name(&self) -> &str {
        &self.application_name
    }

    pub fn application_version(&self) -> &Version {
        &self.application_version
    }
}
