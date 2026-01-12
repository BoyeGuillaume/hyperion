use std::{collections::BTreeMap, sync::Arc};

use parking_lot::RwLock;
use semver::Version;
use uuid::Uuid;

use crate::{
    base::module::ModuleContext,
    ext::{DynPluginEXT, StaticPluginEXT, hylog::LogMessageEXT, load_plugin_by_name},
    hyinfo, hytrace,
    utils::error::HyResult,
};

pub mod api;
pub mod meta;
pub mod module;

build_info::build_info!(fn retrieve_build_info);

/// Container for extension-specific state and callbacks for an instance.
pub struct InstanceStateEXT {
    pub log_callback: RwLock<fn(&InstanceContext, LogMessageEXT)>,
}
impl InstanceStateEXT {}

impl Default for InstanceStateEXT {
    fn default() -> Self {
        Self {
            log_callback: RwLock::new(|_, _| {}),
        }
    }
}

/// Internal instance context that owns modules, extensions, and diagnostics
/// state for a single Hyperion engine instantiation.
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
    pub extensions: BTreeMap<Uuid, Box<dyn DynPluginEXT>>,

    /// Function pointers for logging callbacks
    pub ext: InstanceStateEXT,
}

impl Drop for InstanceContext {
    fn drop(&mut self) {
        hytrace!(self, "Tearing down InstanceContext at {:p}", self);

        while let Some((_uuid, mut ext)) = self.extensions.pop_last() {
            ext.teardown();
        }
    }
}

impl InstanceContext {
    /// Returns the typed plugin reference for the supplied `PluginExtStatic`
    /// implementor, if it was enabled for this instance.
    pub fn get_plugin_ext<T: StaticPluginEXT>(&self) -> Option<&T> {
        self.extensions
            .get(&T::UUID)
            .and_then(|wrapper| wrapper.downcast_ref())
    }

    /// Constructs a new [`InstanceContext`] and wires all enabled extensions
    /// into it. Unsafe because it loads user-provided shared objects.
    pub unsafe fn create(mut instance_create_info: api::InstanceCreateInfo) -> HyResult<Arc<Self>> {
        // Construct state about the application.
        let application_name = instance_create_info.application_info.application_name;
        let application_version = instance_create_info
            .application_info
            .application_version
            .into();
        let engine_version = instance_create_info.application_info.engine_version.into();
        let engine_name = instance_create_info.application_info.engine_name;

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
            ext: Default::default(),
        };

        // For each enabled extension, load and instantiate it.
        for ext_name in &instance_create_info.enabled_extensions {
            let plugin = load_plugin_by_name(ext_name, &mut instance_create_info.ext)?;
            instance.extensions.insert(plugin.uuid(), plugin);
        }

        // Initialize each extension with the instance context.
        let instance = Arc::new_cyclic(|weak| {
            for plugin in instance.extensions.values_mut() {
                plugin.attach_to(weak.clone());
            }
            instance
        });

        // For each plugin, call initialize.
        for plugin in instance.extensions.values() {
            plugin.initialize()?;
        }

        // Logging information about the created instance.
        hytrace!(
            instance,
            "Instance created successfully at {:p}",
            Arc::as_ptr(&instance)
        );
        hyinfo!(
            instance,
            "Application '{}' v{} using engine '{}' v{}",
            &instance.application_name,
            &instance.application_version,
            &instance.engine_name,
            &instance.engine_version
        );
        hyinfo!(
            instance,
            "Loaded {} extensions: {:?}",
            instance.extensions.len(),
            instance_create_info.enabled_extensions,
        );
        hyinfo!(
            instance,
            "hycore version: v{} (features: {:?}) -- {} {}",
            _build_info.crate_info.version,
            _build_info.crate_info.enabled_features,
            _build_info.target.triple,
            _build_info.profile,
        );
        hyinfo!(
            instance,
            "built from commit {}..{} on branch '{}'",
            _build_info
                .version_control
                .as_ref()
                .and_then(|x| x.git())
                .map(|x| x.commit_short_id.clone())
                .unwrap_or_else(|| "00000000".to_string()),
            _build_info
                .version_control
                .as_ref()
                .and_then(|x| x.git())
                .map(|x| if x.dirty { " (dirty)" } else { "" })
                .unwrap_or_else(|| ""),
            _build_info
                .version_control
                .as_ref()
                .and_then(|x| x.git())
                .and_then(|x| x.branch.as_ref())
                .map(|x| x.clone())
                .unwrap_or_else(|| "<unnamed>".to_string())
        );

        Ok(instance)
    }
}
