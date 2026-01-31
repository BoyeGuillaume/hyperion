//! Entry point for formal verification functionalities.

use std::sync::{Arc, Weak};

use downcast_rs::{DowncastSync, impl_downcast};
use hyinstr::attached::AttachedFunction;
use parking_lot::RwLock;
use slotmap::{SlotMap, new_key_type};

use crate::{
    base::{InstanceContext, module::FunctionContext},
    hytrace,
    utils::{
        error::{HyError, HyResult},
        opaque::{OpaqueList, OpaqueObject},
    },
};

pub mod axioms;

/// Base trait for dynamic derivation strategies.
///
/// This trait should not be directly implemented, instead the user
/// should implement [`DerivationStrategy`] which automatically
/// implements this trait.
///
pub trait DynDerivationStrategyBase: Send + Sync {
    /// Returns the name of the derivation strategy.
    ///
    /// This is a unique name used for identification purposes.
    fn name(&self) -> &'static str;
}

/// Dynamic trait for derivation strategies.
pub trait DynDerivationStrategy: DynDerivationStrategyBase + DowncastSync {
    /// Performs derivation.
    fn derive(
        &self,
        context: &FunctionContext,
        attached_function: &RwLock<AttachedFunction>,
        ext: Option<&dyn OpaqueObject>,
    );

    /// Retrieves the unique identifier instance associated with this strategy.
    fn instance(&self) -> &Weak<InstanceContext>;
}
impl_downcast!(sync DynDerivationStrategy);

/// Static, non-dynamic trait for derivation strategies.
pub trait DerivationStrategy: Sized + Send + Sync {
    /// Unique name used to identify the derivation strategy.
    const NAME: &'static str;

    /// Constructs a new instance of the derivation strategy.
    fn new(instance: Weak<InstanceContext>, ext: &mut OpaqueList) -> HyResult<Self>;
}

impl<T: DerivationStrategy> DynDerivationStrategyBase for T {
    fn name(&self) -> &'static str {
        <T as DerivationStrategy>::NAME
    }
}

pub type DerivationStrategyLoaderFn =
    fn(Weak<InstanceContext>, &mut OpaqueList) -> HyResult<Arc<dyn DynDerivationStrategy>>;

/// Inventory containing derivation strategy registrations.
pub struct DerivationStrategyRegistry {
    pub name: &'static str,
    pub loader: DerivationStrategyLoaderFn,
}
inventory::collect!(DerivationStrategyRegistry);

#[macro_export]
macro_rules! register_derivation_strategy {
    (
        $strategy:ty
    ) => {
        $crate::inventory::submit! {
            $crate::formal::DerivationStrategyRegistry {
                name: <$strategy as $crate::formal::DerivationStrategy>::NAME,
                loader: |instance: std::sync::Weak<$crate::base::InstanceContext>, ext: &mut $crate::utils::opaque::OpaqueList| -> $crate::utils::error::HyResult<std::sync::Arc<dyn $crate::formal::DynDerivationStrategy>> {
                    let strategy = <$strategy as $crate::formal::DerivationStrategy>::new(instance, ext)?;
                    Ok(std::sync::Arc::new(strategy))
                },
            }
        }
    };
    () => {};
}

new_key_type! {
    /// Key type for registered derivation strategies.
    pub struct DerivationStrategyKey;
}

/// Library managing registered derivation strategies.
///
/// Notice that multiple similar-strategies can coexist
/// in the library even if they have the same UUID. This is
/// because strategies can be configured through their
/// [`OpaqueList`] during construction.
///
pub struct DerivationStrategyLibrary {
    strategies: SlotMap<DerivationStrategyKey, Arc<dyn DynDerivationStrategy>>,
}

impl DerivationStrategyLibrary {
    /// Adds a new theorem inference strategy to the library by its UUID.
    pub fn add_derivation_by_uuid(
        &mut self,
        instance: &Arc<InstanceContext>,
        name: &str,
        ext: &mut OpaqueList,
    ) -> HyResult<DerivationStrategyKey> {
        for registry in inventory::iter::<DerivationStrategyRegistry> {
            if registry.name == name {
                hytrace!(
                    instance,
                    "Loading derivation strategy with name {} using extensions: {:?}",
                    name,
                    ext
                );
                let strategy = (registry.loader)(Arc::downgrade(instance), ext)?;
                assert!(
                    strategy.name() == name,
                    "Loaded derivation strategy name does not match the requested name"
                );

                let key = self.strategies.insert(strategy);
                return Ok(key);
            }
        }

        Err(HyError::KeyNotFound {
            key: name.to_string(),
            context: "derivation strategy".to_string(),
        })
    }

    /// Adds a new derivation strategy that is not part of the global registry.
    /// This is useful for dynamically created strategies.
    pub fn push(
        &mut self,
        strategy: Arc<dyn DynDerivationStrategy>,
    ) -> HyResult<DerivationStrategyKey> {
        let key = self.strategies.insert(strategy);
        Ok(key)
    }

    /// Removes a derivation strategy from the library by its key.
    pub fn remove(&mut self, key: DerivationStrategyKey) -> HyResult<()> {
        if self.strategies.remove(key).is_some() {
            Ok(())
        } else {
            Err(HyError::KeyNotFound {
                key: format!("{:?}", key),
                context: "DerivationStrategyLibrary::remove".to_string(),
            })
        }
    }

    /// Retrieves a reference to a registered derivation strategy by its name.
    pub fn get(&self, key: DerivationStrategyKey) -> Option<&Arc<dyn DynDerivationStrategy>> {
        self.strategies.get(key)
    }
}
