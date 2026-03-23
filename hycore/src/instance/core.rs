use std::{collections::BTreeMap, sync::Arc};

use bevy_ecs::{component::Component, entity::Entity};
use hyinstr::modules::{Function, Global, symbol::ExternalFunction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleHandle(pub(crate) Entity);

impl ModuleHandle {
    pub fn get(&self) -> Entity {
        self.0
    }
}

/// All globals/functions/external functions of a module
/// are stored in child entities of the module entity
#[derive(Component, Default)]
pub struct ModuleComponent {
    pub globals: BTreeMap<Uuid, Entity>,
    pub functions: BTreeMap<Uuid, Entity>,
    pub external_functions: BTreeMap<Uuid, Entity>,
}

/// Component equivalent to a [`Function`] to store in the ECS
#[derive(Component)]
pub struct FunctionComponent {
    pub inner: Arc<Function>,
}

impl std::ops::Deref for FunctionComponent {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Component equivalent to a [`Global`] to store in the ECS
#[derive(Component)]
pub struct GlobalComponent {
    pub inner: Global,
}

impl std::ops::Deref for GlobalComponent {
    type Target = Global;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for GlobalComponent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Component equivalent to a [`ExternalFunction`] to store in the ECS
#[derive(Component)]
pub struct ExternalFunctionComponent {
    pub inner: ExternalFunction,
}

impl std::ops::Deref for ExternalFunctionComponent {
    type Target = ExternalFunction;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for ExternalFunctionComponent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
