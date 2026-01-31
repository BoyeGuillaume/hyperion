use std::{collections::BTreeMap, sync::Arc};

use dashmap::DashMap;
use hyinstr::{modules::Module, types::TypeRegistry};
use parking_lot::RwLock;
use semver::Version;
use slotmap::{Key, SlotMap, new_key_type};
use uuid::Uuid;

use crate::{
    base::module::{FunctionContext, ModuleContext},
    ext::{DynPluginEXT, StaticPluginEXT, hylog::LogMessageEXT, load_plugin_by_name},
    hyerror, hyinfo, hytrace,
    theorems::library::TheoremLibrary,
    utils::error::{HyError, HyResult},
};

pub mod api;
pub mod meta;
pub mod module;

build_info::build_info!(fn retrieve_build_info);

/// Container for extension-specific state and callbacks for an instance.
pub struct InstanceStateEXT {
    pub log_callback: RwLock<fn(&InstanceContext, LogMessageEXT)>,
}

impl Default for InstanceStateEXT {
    fn default() -> Self {
        Self {
            log_callback: RwLock::new(|_, _| {}),
        }
    }
}

new_key_type! {
    pub struct ModuleKey;
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
    pub node_id: u32,

    /// A list of modules loaded into this instance
    pub modules: RwLock<SlotMap<ModuleKey, Arc<ModuleContext>>>,

    /// A list of all extension (by UUID) loaded into this instance.
    pub extensions: BTreeMap<Uuid, Box<dyn DynPluginEXT>>,

    /// Function pointers for logging callbacks
    pub ext: InstanceStateEXT,

    /// Type registry associated with this instance
    pub type_registry: TypeRegistry,

    /// UUID timestamp context for this instance
    context: uuid::timestamp::context::Context,
    node_id_bytes: [u8; 6],
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
    pub fn generate_uuid(&self) -> Uuid {
        let ts = uuid::Timestamp::now(&self.context);
        Uuid::new_v6(ts, &self.node_id_bytes)
    }

    /// Returns the typed plugin reference for the supplied `PluginExtStatic`
    /// implementor, if it was enabled for this instance.
    pub fn get_plugin_ext<T: StaticPluginEXT>(&self) -> Option<&T> {
        self.extensions
            .get(&T::UUID)
            .and_then(|wrapper| wrapper.downcast_ref())
    }

    /// Constructs a new [`InstanceContext`] and wires all enabled extensions
    /// into it.
    pub fn create(mut instance_create_info: api::InstanceCreateInfo) -> HyResult<Arc<Self>> {
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
        let node_id: [u8; 6] = {
            let partial_node_id = instance_create_info.node_id.to_le_bytes();
            let mut node_id = [0u8; 6];
            node_id[0..4].copy_from_slice(&partial_node_id);
            node_id
        };
        let mut instance = InstanceContext {
            version: _build_info.crate_info.version.clone(),
            node_id: instance_create_info.node_id,
            application_name,
            application_version,
            engine_name,
            engine_version,
            modules: Default::default(),
            extensions: BTreeMap::new(),
            ext: Default::default(),
            type_registry: TypeRegistry::new(node_id),
            context: uuid::timestamp::context::Context::new_random(),
            node_id_bytes: node_id,
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
            "Application '{}' v{} using engine '{}' v{} (node ID: {})",
            &instance.application_name,
            &instance.application_version,
            &instance.engine_name,
            &instance.engine_version,
            &instance.node_id
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
                .cloned()
                .unwrap_or_else(|| "<unnamed>".to_string())
        );

        Ok(instance)
    }

    pub fn add_module(self: &Arc<Self>, module: Module) -> HyResult<ModuleKey> {
        // Verify and type check the module before adding it to the instance
        hytrace!(self, "Verifying module before adding to instance");
        module.verify()?;

        for func in module.functions.values() {
            hytrace!(
                self,
                "Type checking function '{}'",
                func.name
                    .clone()
                    .unwrap_or_else(|| format!("@{}", func.uuid))
            );
            func.type_check(&self.type_registry).inspect_err(|e| {
                hyerror!(
                    self,
                    "Type check failed for function '{}': {}",
                    func.name
                        .clone()
                        .unwrap_or_else(|| format!("@{}", func.uuid)),
                    e
                );
            })?;
        }

        let weak_self = Arc::downgrade(self);

        // Create a new module context
        let uuid = self.generate_uuid();
        let mut module_context = ModuleContext {
            uuid,
            module,
            library: TheoremLibrary::new(),
            funcs: DashMap::new(),
            key: ModuleKey::null(),
            instance: weak_self,
        };

        // Populate function contexts
        for function in module_context.module.functions.values() {
            let func_ctx = FunctionContext::new(function.clone());
            module_context.funcs.insert(func_ctx.uuid, func_ctx);
        }

        // Add the module context to the instance
        hyinfo!(
            self,
            "Module with UUID {} added to instance. Contains functions: {:?}",
            module_context.uuid,
            module_context
                .module
                .functions
                .values()
                .map(|v| v.name.clone().unwrap_or_else(|| format!("@{}", v.uuid)))
                .collect::<Vec<_>>()
        );
        let mut modules = self.modules.write();
        let key = modules.insert_with_key(|key| {
            module_context.key = key;
            hytrace!(
                self,
                "ModuleContext inserted into InstanceContext with key {:?}",
                key
            );
            Arc::new(module_context)
        });

        Ok(key)
    }

    pub fn get_module_by_key(&self, key: ModuleKey) -> Option<Arc<ModuleContext>> {
        let modules = self.modules.read();
        modules.get(key).cloned()
    }

    pub fn get_module_by_uuid(&self, uuid: Uuid) -> Option<Arc<ModuleContext>> {
        let modules = self.modules.read();
        modules.values().find(|m| m.uuid == uuid).cloned()
    }

    pub fn remove_module_by_key(&self, key: ModuleKey) -> HyResult<()> {
        let mut modules = self.modules.write();
        if modules.remove(key).is_some() {
            hyinfo!(self, "Module with key {:?} removed from instance", key);
            Ok(())
        } else {
            hyinfo!(
                self,
                "Attempted to remove module with key {:?}, but it was not found",
                key
            );
            Err(HyError::KeyNotFound {
                key: format!("{:?}", key),
                context: "InstanceContext::remove_module_by_key".to_string(),
            })
        }
    }
}
