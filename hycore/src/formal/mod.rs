//! Entry point for formal verification functionalities.

use std::{collections::BTreeMap, sync::Weak};

use uuid::Uuid;

use crate::{
    base::InstanceContext,
    utils::error::{HyError, HyResult},
};

/// Base trait for dynamic theorem inference strategies. Should not be
/// implemented directly, instead prefer implementing [`TheoremInferenceStrategy`].
pub trait DynTheoremInferenceStrategyBase: Send + Sync {
    // Returns the unique identifier for the theorem inference strategy.
    fn uuid(&self) -> Uuid;
}

pub trait DynTheoremInferenceStrategy: DynTheoremInferenceStrategyBase {
    fn derive(&self);
}

/// Static, non-dynamic trait for theorem inference strategies.
pub trait TheoremInferenceStrategy: Sized + Send + Sync {
    /// Unique identifier for the theorem inference strategy.
    const UUID: Uuid;

    /// Constructs a new instance of the theorem inference strategy.
    fn new(instance: Weak<InstanceContext>) -> HyResult<Self>;
}

impl<T: TheoremInferenceStrategy> DynTheoremInferenceStrategyBase for T {
    fn uuid(&self) -> Uuid {
        <T as TheoremInferenceStrategy>::UUID
    }
}

/// Inventory containing theorem inference strategy registrations.
pub struct TheoremInferenceStrategyRegistry {
    pub uuid: Uuid,
    pub loader: fn(Weak<InstanceContext>) -> HyResult<Box<dyn DynTheoremInferenceStrategy>>,
}
inventory::collect!(TheoremInferenceStrategyRegistry);

#[macro_export]
macro_rules! register_theorem_inference_strategy {
    (
        $strategy:ty
    ) => {
        $crate::inventory::submit! {
            $crate::formal::TheoremInferenceStrategyRegistry {
                uuid: <$strategy as $crate::formal::TheoremInferenceStrategy>::UUID,
                loader: |instance: std::sync::Weak<$crate::base::InstanceContext>| -> $crate::utils::error::HyResult<Box<dyn $crate::formal::DynTheoremInferenceStrategy>> {
                    let strategy = <$strategy as $crate::formal::TheoremInferenceStrategy>::new(instance)?;
                    Ok(Box::new(strategy))
                },
            }
        }
    };
    () => {};
}

/// Library managing registered theorem inference strategies.
pub struct TheoremInferenceStrategyLibrary {
    strategies: BTreeMap<Uuid, Box<dyn DynTheoremInferenceStrategy>>,
}

impl TheoremInferenceStrategyLibrary {
    /// Adds a new theorem inference strategy to the library by its UUID.
    pub fn add_strategy_by_uuid(
        &mut self,
        uuid: Uuid,
        instance: Weak<InstanceContext>,
    ) -> HyResult<()> {
        for registry in inventory::iter::<TheoremInferenceStrategyRegistry> {
            if registry.uuid == uuid {
                let strategy = (registry.loader)(instance)?;
                self.strategies.insert(uuid, strategy);
                return Ok(());
            }
        }

        Err(HyError::KeyNotFound {
            key: uuid.to_string(),
            context: "theorem inference strategy".to_string(),
        })
    }

    /// Retrieves a reference to a registered theorem inference strategy by its UUID.
    pub fn get_strategy(&self, uuid: &Uuid) -> Option<&Box<dyn DynTheoremInferenceStrategy>> {
        self.strategies.get(uuid)
    }
}
