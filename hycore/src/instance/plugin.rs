use downcast_rs::{DowncastSync, impl_downcast};
use smallbox::{SmallBox, space};

use crate::{HyResult, ext::ExtList, instance::Instance};

/// Plugins configure the behavior of the Hyperion instance
///
/// We chose not to use bevy's plugin system because their system is designed to be
/// use for application and game development. We therefore design our own plugin system
/// heavily inspired by bevy's, but with some differences.
pub trait Plugin: DowncastSync {
    /// The name of the plugin, should be unique across all plugins
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// First step toward plugin initialization
    fn init(&mut self, instance: &mut Instance, ext: Option<&mut ExtList>) -> HyResult<()>;

    /// Has the plugin finished its setup?
    ///
    /// Once the [`Self::init`] method is called, the plugin won't be considered ready until this method
    /// returns `true`. Useful when asynchronous setup is required
    fn ready(&self) -> bool {
        true
    }

    /// Whether this plugin is user registered in the inventory or not
    ///
    /// Plugin that are in the [`PublicPluginInventory`] are considered public,
    /// they should **ONLY** be registered by the user through [`super::InstanceCreateInfo::enabled_plugins`] and
    /// not directly added programmatically.
    ///
    /// However, plugin that are not in the inventory are considered private, and they can be added programmatically
    /// notably when plugin are dependent on another plugin. If a public is public and is not registered in the inventory,
    /// you should return an error in the [`Self::init`] method
    fn is_public(&self) -> bool {
        false
    }

    /// Finish adding this plugin **once every other plugin is ready**
    ///
    /// Finish adding this plugin to the App, once all plugins registered are ready. This can be useful for plugins
    /// that depends on another plugin asynchronous setup, like the renderer.
    ///
    /// Because of rust borrowing rules, [`Instance::has_plugin`] on self will return false, but this is only a temporary
    /// state to avoid the need for interior mutability in the plugin struct.
    fn finish(&mut self, _instance: &mut Instance) {}

    /// Cleanup the plugin before the instance is dropped
    ///
    /// The cleaning up of plugin is in reverse order of their initialization
    fn cleanup(&mut self, _instance: &mut Instance) {}
}
impl_downcast!(sync Plugin);

/// Constructor trait for plugins, used for the inventory
pub trait PluginConstructor: Plugin + Sized {
    fn new(ext: &mut ExtList) -> HyResult<Self>;

    fn new_boxed(ext: &mut ExtList) -> HyResult<DynPlugin> {
        Ok(smallbox::smallbox!(Self::new(ext)?))
    }
}

impl<T: Default + Plugin + Sized> PluginConstructor for T {
    fn new(_ext: &mut ExtList) -> HyResult<Self> {
        Ok(Self::default())
    }
}

/// Type erased plugin type, used for storing plugins in the instance
pub type DynPlugin = SmallBox<dyn Plugin, space::S4>;

/// Plugin registry using the inventory crate
pub struct PublicPluginInventory {
    // pub name: &'static str,
    pub name: fn() -> &'static str,
    pub type_id: std::any::TypeId,
    pub constructor: fn(&mut ExtList) -> HyResult<DynPlugin>,
}
inventory::collect!(PublicPluginInventory);

/// A macro to register a plugin in the inventory
#[macro_export]
macro_rules! register_plugin {
    (
        $plugin_type:ty
    ) => {
        crate::inventory::submit! {
            crate::instance::plugin::PublicPluginInventory {
                name: || std::any::type_name::<$plugin_type>(), // unstable const #63084
                type_id: std::any::TypeId::of::<$plugin_type>(),
                constructor:
                    <$plugin_type as crate::instance::plugin::PluginConstructor>::new_boxed,
            }
        }
    };
}
