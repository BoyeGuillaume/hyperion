use bevy_ecs::prelude::*;
use hyinstr::types::TypeRegistry;

/// A resource that holds the [`TypeRegistry`] so that it can be accessed by plugins and systems
#[derive(Resource)]
pub struct TypeRegistryRes {
    pub type_registry: TypeRegistry,
}

/// A resource that holds the current module being analyzed
#[derive(Resource)]
pub struct CurrentModuleFilter {
    pub entity: Entity,
}
