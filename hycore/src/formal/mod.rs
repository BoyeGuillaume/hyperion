//! Entry point for formal verification functionalities.

use std::{collections::BTreeMap, sync::Weak};

use uuid::Uuid;

use crate::{
    base::InstanceContext,
    utils::error::{HyError, HyResult},
};

/// Base trait for dynamic derivation strategies.
///
/// This trait should not be directly implemented, instead the user
/// should implement [`DerivationStrategy`] which automatically
/// implements this trait.
///
pub trait DynDerivationStrategyBase: Send + Sync {
    /// Returns the unique identifier for the derivation strategy.
    ///
    /// See [`DerivationStrategy::UUID`]
    fn uuid(&self) -> Uuid;
}

/// Dynamic trait for derivation strategies.
pub trait DynDerivationStrategy: DynDerivationStrategyBase {
    /// Performs derivation.
    ///
    /// This is a placeholder method and should be implemented with actual logic.
    fn derive(&self);
}

/// Static, non-dynamic trait for derivation strategies.
pub trait DerivationStrategy: Sized + Send + Sync {
    /// Unique identifier for the derivation strategy.
    const UUID: Uuid;

    /// Constructs a new instance of the derivation strategy.
    fn new(instance: Weak<InstanceContext>) -> HyResult<Self>;
}

impl<T: DerivationStrategy> DynDerivationStrategyBase for T {
    fn uuid(&self) -> Uuid {
        <T as DerivationStrategy>::UUID
    }
}

/// Inventory containing derivation strategy registrations.
pub struct DerivationStrategyRegistry {
    pub uuid: Uuid,
    pub loader: fn(Weak<InstanceContext>) -> HyResult<Box<dyn DynDerivationStrategy>>,
}
inventory::collect!(DerivationStrategyRegistry);

#[macro_export]
macro_rules! register_derivation_strategy {
    (
        $strategy:ty
    ) => {
        $crate::inventory::submit! {
            $crate::formal::DerivationStrategyRegistry {
                uuid: <$strategy as $crate::formal::DerivationStrategy>::UUID,
                loader: |instance: std::sync::Weak<$crate::base::InstanceContext>| -> $crate::utils::error::HyResult<Box<dyn $crate::formal::DynDerivationStrategy>> {
                    let strategy = <$strategy as $crate::formal::DerivationStrategy>::new(instance)?;
                    Ok(Box::new(strategy))
                },
            }
        }
    };
    () => {};
}

/// Library managing registered derivation strategies.
///
/// This is similar to [`DerivationStrategyRegistry`], but whereas
/// the registry is static and global, the library is instantiated
/// per instance context.
/// This enables extensions to register only the strategies they need,
/// reducing memory usage and potential conflicts. Add improving
/// modularity.
///
pub struct DerivationStrategyLibrary {
    strategies: BTreeMap<Uuid, Box<dyn DynDerivationStrategy>>,
}

impl DerivationStrategyLibrary {
    /// Adds a new theorem inference strategy to the library by its UUID.
    pub fn add_derivation_by_uuid(
        &mut self,
        uuid: Uuid,
        instance: Weak<InstanceContext>,
    ) -> HyResult<()> {
        for registry in inventory::iter::<DerivationStrategyRegistry> {
            if registry.uuid == uuid {
                let strategy = (registry.loader)(instance)?;
                assert!(
                    strategy.uuid() == uuid,
                    "Loaded derivation strategy UUID does not match the requested UUID"
                );

                if self.strategies.contains_key(&uuid) {
                    return Err(HyError::DuplicatedKey {
                        key: uuid.to_string(),
                        context: "derivation strategy".to_string(),
                    });
                }

                self.strategies.insert(uuid, strategy);
                return Ok(());
            }
        }

        Err(HyError::KeyNotFound {
            key: uuid.to_string(),
            context: "derivation strategy".to_string(),
        })
    }

    /// Adds a new derivation strategy that is not part of the global registry.
    /// This is useful for dynamically created strategies.
    pub fn add_derivation_strategy(
        &mut self,
        strategy: Box<dyn DynDerivationStrategy>,
    ) -> HyResult<()> {
        let uuid = strategy.uuid();
        if self.strategies.contains_key(&uuid) {
            return Err(HyError::DuplicatedKey {
                key: uuid.to_string(),
                context: "derivation strategy".to_string(),
            });
        }

        self.strategies.insert(uuid, strategy);
        Ok(())
    }

    /// Retrieves a reference to a registered derivation strategy by its UUID.
    pub fn get(&self, uuid: &Uuid) -> Option<&Box<dyn DynDerivationStrategy>> {
        self.strategies.get(uuid)
    }
}
