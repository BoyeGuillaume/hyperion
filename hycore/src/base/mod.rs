use std::{collections::BTreeMap, sync::Arc};

use semver::Version;
use uuid::Uuid;

use crate::{
    base::{
        ext::{PluginExtWrapper, load_plugin_by_name},
        meta::HyperionMetaInfo,
        module::ModuleContext,
    },
    utils::error::HyResult,
};

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
    pub engine_name: String,

    /// A list of modules loaded into this instance
    pub modules: Vec<ModuleContext>,

    /// A list of all extension (by UUID) loaded into this instance.
    pub extensions: BTreeMap<Uuid, PluginExtWrapper>,
}

impl InstanceContext {
    pub unsafe fn create(instance_create_info: &api::InstanceCreateInfo) -> HyResult<Arc<Self>> {
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
            .to_string();

        // Open the metadata containing build info.
        let meta_path = HyperionMetaInfo::default_path();
        let meta_info = HyperionMetaInfo::load_from_toml(&meta_path)?;

        // Retrieve build info for the current crate.
        let _build_info = retrieve_build_info();

        // Attempt to instantiate modules for each enabled extension.
        let mut instance = InstanceContext {
            version: _build_info.crate_info.version.clone(),
            application_name,
            application_version,
            engine_name,
            engine_version,
            modules: Vec::new(),
            extensions: BTreeMap::new(),
        };

        // For each enabled extension, load and instantiate it.
        for &ext_name in instance_create_info.enabled_extensions {
            let plugin = unsafe {
                load_plugin_by_name(&meta_info, ext_name, _build_info.crate_info.version.clone())?
            };
            instance.extensions.insert(plugin.uuid(), plugin);
        }

        // Initialize each extension with the instance context.
        let instance = Arc::new(instance);
        // Arc::new_cyclic(data_fn)

        Ok(instance)
    }
}
