use bevy_ecs::prelude::*;
use hyinstr::types::TypeRegistry;
use parking_lot::RwLock;
use smallbox::{SmallBox, smallbox, space};

use crate::{
    HyError, HyResult,
    api::InstanceCreateInfo,
    instance::plugin::{DynPlugin, Plugin},
    inventory, register_plugin,
    resource::TypeRegistryRes,
};

pub mod plugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstanceState {
    /// Initial state of the instance, during creation and plugin initialization.
    Initializing,

    /// Instance has been fully initialized, and all plugins are ready.
    Ready,

    /// Instance is being dropped, plugins are being cleaned up.
    Dropping,
}

/// The [`Instance`] struct is an internal object that represents a single instance of the Hyperion library
/// runtime. It should be an opaque object that is never directly accessed by the user of the library
pub struct Instance {
    pub world: RwLock<World>,

    plugins: Vec<SmallBox<dyn Plugin, space::S4>>,
    state: InstanceState,
}

impl Instance {
    #[inline]
    pub fn get<T: Plugin + 'static>(&self) -> Option<&T> {
        self.plugins
            .iter()
            .find(|p| p.type_id() == std::any::TypeId::of::<T>())
            .and_then(|p| p.downcast_ref::<T>())
    }

    #[inline]
    pub fn has_plugin<T: Plugin + 'static>(&self) -> bool {
        self.plugins
            .iter()
            .any(|p| p.type_id() == std::any::TypeId::of::<T>())
    }

    #[inline]
    fn internal_world_init(world: &mut World, create_info: InstanceCreateInfo<'_>) -> HyResult<()> {
        // Add the type registry resource to the world, so that it can be accessed by plugins and systems
        let node_id = {
            let node_id = create_info.node_rank.to_ne_bytes();
            let mut node_id_truncated = [0u8; 6];
            node_id_truncated.copy_from_slice(&node_id[0..6]);
            if node_id[6..].iter().any(|&b| b != 0) {
                return Err(HyError::msg(format!(
                    "Only the 6 least significant bytes of the node rank are used, but the provided node rank has non-zero bytes in the ignored part (provided node rank was 0x{:08x})",
                    create_info.node_rank,
                )));
            }

            node_id_truncated
        };

        world.insert_resource(TypeRegistryRes {
            type_registry: TypeRegistry::new(node_id),
        });

        // At this point we consider the world to be fully initialized, we can now call the `build` method of each plugin
        Ok(())
    }

    #[inline]
    fn internal_add_plugin(
        &mut self,
        mut plugin: DynPlugin,
        should_be_public: bool,
    ) -> HyResult<()> {
        // Check that the instance is still in the initializing state, as we don't want to add plugin after the instance is ready
        if self.state != InstanceState::Initializing {
            return Err(HyError::msg(format!(
                "Plugins cannot be registered after the instance is ready, but plugin '{}' was added anyway",
                plugin.name()
            )));
        }

        // Check that the plugin is only added once
        if self.plugins.iter().any(|p| p.type_id() == plugin.type_id()) {
            return Err(HyError::msg(format!(
                "Plugin '{}' has already been registered, make sure to only add a plugin once",
                plugin.name()
            )));
        }

        // Verify that the plugin is public if it should be, and private if it shouldn't be
        if plugin.is_public() != should_be_public {
            return Err(HyError::msg(format!(
                "Plugin '{}' is {} but was added as {}",
                plugin.name(),
                if plugin.is_public() {
                    "public"
                } else {
                    "private"
                },
                if should_be_public {
                    "publically"
                } else {
                    "programmatically"
                },
            )));
        }

        plugin.init(self);
        self.plugins.push(plugin);
        Ok(())
    }

    /// Add a plugin to the instance
    ///
    /// - This should only be used for programmatically adding private plugin. See [`Plugin::is_public`] for
    ///   more details on public vs private plugin.
    /// - Should not be called after the instance is ready, as the `finish` method of the plugin won't be called
    ///
    #[inline]
    pub fn add_plugin<T: Plugin + 'static>(&mut self, plugin: T) -> HyResult<()> {
        self.internal_add_plugin(smallbox!(plugin), false)
    }

    /// Create a new instance of the Hyperion library runtime, given an [`InstanceCreateInfo`] struct
    pub fn new(create_info: InstanceCreateInfo<'_>) -> HyResult<Self> {
        // Create the instance with the initial state, and an empty world and plugin list
        let mut instance = Instance {
            world: RwLock::new(World::new()),
            plugins: Vec::new(),
            state: InstanceState::Initializing,
        };

        let mut world = instance.world.write();
        Self::internal_world_init(&mut world, create_info)?;
        drop(world);

        // Add public plugins specified in the create info, by looking them up in the inventory
        for enabled_plugin_name in create_info.enabled_plugins {
            let constructor = inventory::iter::<plugin::PublicPluginInventory>
                .into_iter()
                .find(|constructor| constructor.name == *enabled_plugin_name)
                .ok_or_else(|| {
                    HyError::msg(format!(
                        "InstanceCreateInfo specifies enabled plugin '{}' which is not registered in the inventory. Possible values are: {}",
                        enabled_plugin_name,
                        inventory::iter::<plugin::PublicPluginInventory>
                            .into_iter()
                            .map(|constructor| constructor.name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })?;
            let plugin = (constructor.constructor)();

            // Verify that the plugin returned by the constructor has the correct type and name, to avoid mistakes in the implementation of the constructor
            if constructor.type_id != plugin.type_id() {
                return Err(HyError::msg(format!(
                    "Plugin constructor for plugin '{}' returned a plugin of the wrong type. This is likely an internal bug, please report it to the developers",
                    enabled_plugin_name
                )));
            }

            if constructor.name != plugin.name() {
                return Err(HyError::msg(format!(
                    "Plugin constructor for plugin '{}' returned a plugin with a different name ('{}'). This is likely an internal bug, please report it to the developers",
                    enabled_plugin_name,
                    plugin.name()
                )));
            }

            instance.internal_add_plugin(plugin, true)?;
        }

        // Transition state to ready, so that no more plugin can be added, and call the `finish` method of each plugin
        instance.state = InstanceState::Ready;

        let plugin_count = instance.plugins.len(); // Cannot be changed because state is now set to ready
        for plugin_index in 0..plugin_count {
            // 1. Swap the plugin at the current index with a dummy plugin
            let mut plugin: SmallBox<dyn Plugin, space::S4> = smallbox!(_InstanceDummyPlugin); // No allocation due to smallbox
            std::mem::swap(&mut plugin, &mut instance.plugins[plugin_index]);

            // 2. Call the `finish` method of the plugin, with the instance as argument
            plugin.finish(&mut instance);

            // 3. Put the plugin back in the plugin list
            std::mem::swap(&mut plugin, &mut instance.plugins[plugin_index]);
        }

        // Finally return the instance
        Ok(instance)
    }
}

impl std::ops::Drop for Instance {
    #[inline]
    fn drop(&mut self) {
        // Transition state to dropping, so that no more plugin can be added, and call the `cleanup` method of each plugin
        self.state = InstanceState::Dropping;

        // Cleanup plugins in reverse order of their initialization, as some plugin might depend on another plugin, and we
        // want to make sure that the dependent plugin is cleaned up before the plugin it depends on.
        while let Some(mut plugin) = self.plugins.pop() {
            plugin.cleanup(self);
        }
    }
}

/// A dummy plugin used as a placeholder during instance initialization, just ignore
#[derive(Default)]
struct _InstanceDummyPlugin;
impl Plugin for _InstanceDummyPlugin {
    fn is_public(&self) -> bool {
        false
    }

    fn init(&mut self, _instance: &mut Instance) {}
}

register_plugin!(_InstanceDummyPlugin, "_InstanceDummyPlugin");
