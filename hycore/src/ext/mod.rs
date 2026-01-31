//! Runtime primitives for integrating dynamically loaded Hyperion extensions.
//!
//! The module bridges three worlds:
//!
//! - **Metadata** captured by [`HyperionMetaInfo`](crate::base::meta::HyperionMetaInfo), which tells
//!   us where each shared object lives and which UUID it exposes.
//! - **Plugin crates**, which use [`define_plugin!`] to emit the entrypoint, loader, compatibility,
//!   and teardown symbols the host probes right after calling `dlopen`.
//! - **Runtime abstractions** (`PluginExt`, `PluginExtStatic`, `PluginExtWrapper`) consumed by
//!   [`InstanceContext`](crate::base::InstanceContext) when building an engine.
//!
//! Between those pieces the host can safely preload libraries, wire per-instance configuration via
//! [`ExtList`](crate::utils::conf::ExtList), and drop extensions deterministically. Refer to
//! `docs/PluginSystem.md` for an end-to-end narrative.
use std::sync::Weak;

use downcast_rs::{DowncastSync, impl_downcast};
use uuid::Uuid;

use crate::{
    base::InstanceContext,
    utils::{
        error::{HyError, HyResult},
        opaque::OpaqueList,
    },
};

pub mod hylog;

/// Function signature to construct a plugin instance from a loaded shared object.
pub type ExtLoaderFn = fn(&mut OpaqueList) -> HyResult<Box<dyn DynPluginEXT>>;

/// Static registration entry describing one concrete plugin implementation.
pub struct PluginRegistry {
    pub uuid: Uuid,
    pub name: &'static str,
    pub description: &'static str,
    pub loader: ExtLoaderFn,
}
inventory::collect!(PluginRegistry);

/// Macro to define and register a Hyperion plugin.
#[macro_export]
macro_rules! register_plugin {
    (
        $plugin:ty
    ) => {
        $crate::inventory::submit! {
            $crate::ext::PluginRegistry {
                uuid: <$plugin as $crate::ext::StaticPluginEXT>::UUID,
                name: <$plugin as $crate::ext::StaticPluginEXT>::NAME,
                description: <$plugin as $crate::ext::StaticPluginEXT>::DESCRIPTION,
                loader: |ext_list: &mut $crate::utils::opaque::OpaqueList| -> $crate::utils::error::HyResult<Box<dyn $crate::ext::DynPluginEXT>> {
                    let plugin = <$plugin as $crate::ext::StaticPluginEXT>::new(ext_list);
                    Ok(Box::new(plugin))
                },
            }
        }
    };
}

/// Loads a plugin by its registered name from the inventory table.
///
/// # Errors
///
/// Returns [`HyError::PluginNotFound`] when no plugin entry matches `name`.
pub fn load_plugin_by_name(
    name: &str,
    ext_list: &mut OpaqueList,
) -> HyResult<Box<dyn DynPluginEXT>> {
    for plugin in inventory::iter::<PluginRegistry> {
        if plugin.name == name {
            return (plugin.loader)(ext_list);
        }
    }

    Err(HyError::PluginNotFound(name.to_string()))
}

/// Plugin trait defining the lifecycle hooks and behaviors every Hyperion
/// extension must implement.
pub trait PluginEXT {
    /// Provide a handle to the plugin's instance context. This is always called
    /// before [`PluginExt::initialize`].
    fn attach_to(&mut self, instance: Weak<InstanceContext>);

    /// Initializes the plugin for a particular [`InstanceContext`]. This is called
    /// once per instance after loading. [`PluginExt`] is always instantiated only once
    /// per process, so any per-instance state must be set up here.
    fn initialize(&self) -> HyResult<()>;

    /// Tears down the plugin, releasing any resources held. This is called
    /// once per instance when the instance is being destroyed.
    fn teardown(&mut self);
}

impl<T: PluginEXT + StaticPluginEXT> DynPluginEXT for T {
    fn uuid(&self) -> Uuid {
        T::UUID
    }

    fn name(&self) -> &str {
        T::NAME
    }

    fn description(&self) -> &str {
        T::DESCRIPTION
    }
}

/// Runtime contract that every Hyperion plugin must honor.
pub trait DynPluginEXT: DowncastSync + PluginEXT {
    /// Returns the globally unique identifier registering the plugin in
    /// Hyperion metadata files.
    fn uuid(&self) -> Uuid;

    /// Human-readable name exposed to CLI tools and logging.
    fn name(&self) -> &str;

    /// Longer description explaining the functionality or requirements of the
    /// plugin.
    fn description(&self) -> &str;
}
impl_downcast!(sync DynPluginEXT);

/// Compile-time helpers that let the host instantiate plugins without knowing
/// concrete types.
pub trait StaticPluginEXT: DynPluginEXT {
    /// UUID baked into the binary. Must match the value stored in the metadata
    /// TOML file.
    const UUID: Uuid;

    /// Canonical extension name reported during discovery.
    const NAME: &'static str;

    /// Human readable description surfaced to hosts.
    const DESCRIPTION: &'static str;

    /// Constructs a fresh plugin instance. Implementations should keep
    /// allocation lightweight because this is called on every load.
    fn new(ext: &mut OpaqueList) -> Self;
}
