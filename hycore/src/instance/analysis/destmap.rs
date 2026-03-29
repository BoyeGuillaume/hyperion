use crate::{
    hyerror,
    instance::{
        core::{FunctionComponent, ModuleComponent},
        plugin::Plugin,
    },
    plugin::logger::LoggerStateRes,
    resource::CurrentModuleFilter,
    schedule::PostUpdate,
};
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use hyinstr::modules::{InstructionRef, operand::Name};
use std::collections::BTreeMap;

#[derive(Component, Clone, Reflect)]
#[reflect(opaque)]
pub struct DestMap {
    pub dest_map: BTreeMap<Name, InstructionRef>,
}

pub fn derive_dest_map_system(
    query: Query<(Entity, &FunctionComponent), Without<DestMap>>,
    module_query: Query<&Children, With<ModuleComponent>>,
    module_res: Res<CurrentModuleFilter>,
    logger: Res<LoggerStateRes>,
    mut commands: Commands,
) {
    // Find the module corresponding to our query
    let module_children = match module_query.get(module_res.entity) {
        Ok(children) => children,
        Err(_) => {
            hyerror!(
                logger;
                "Failed to find module for DestMap derivation. Skipping DestMap derivation."
            );
            return;
        }
    };

    // Build a mapping from function entities to their corresponding labels
    for (entity, func) in query.iter_many(module_children) {
        // Build the DestMap for this function
        let dest_map = func.derive_dest_map();

        // Insert the DestMap as a component
        commands.entity(entity).insert(DestMap { dest_map });
    }
}

pub struct DeriveDestMapPlugin;
impl Plugin for DeriveDestMapPlugin {
    fn init(
        &mut self,
        instance: &mut crate::instance::Instance,
        _ext: Option<&mut crate::ext::ExtList>,
    ) -> crate::HyResult<()> {
        instance.add_systems(PostUpdate, derive_dest_map_system);
        Ok(())
    }
}
