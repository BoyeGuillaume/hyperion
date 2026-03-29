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
use hyinstr::modules::operand::{Label, Operand};
use petgraph::prelude::DiGraphMap;

#[derive(Component, Clone, Reflect)]
#[reflect(opaque)]
pub struct ControlFlowGraph {
    pub cfg: DiGraphMap<Label, Option<Operand>>,
}

pub fn derive_cfg_system(
    query: Query<(Entity, &FunctionComponent), Without<ControlFlowGraph>>,
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
                "Failed to find module for CFG derivation. Skipping CFG derivation."
            );
            return;
        }
    };

    // Build a mapping from function entities to their corresponding labels
    for (entity, func) in query.iter_many(module_children) {
        // Build the CFG for this function
        let cfg = func.derive_function_flow();

        // Insert the CFG as a component
        commands.entity(entity).insert(ControlFlowGraph { cfg });
    }
}

pub struct DeriveCfgPlugin;
impl Plugin for DeriveCfgPlugin {
    fn init(
        &mut self,
        instance: &mut crate::instance::Instance,
        _ext: Option<&mut crate::ext::ExtList>,
    ) -> crate::HyResult<()> {
        instance.add_systems(PostUpdate, derive_cfg_system);
        Ok(())
    }
}
