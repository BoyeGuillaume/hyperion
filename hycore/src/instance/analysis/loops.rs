use std::collections::BTreeSet;

use crate::{
    hyerror,
    instance::{
        analysis::cfg::{ControlFlowGraph, derive_cfg_system},
        core::FunctionComponent,
        plugin::Plugin,
    },
    plugin::logger::LoggerStateRes,
    resource::CurrentModuleFilter,
    schedule::PostUpdate,
};
use bevy_ecs::prelude::*;
use hyinstr::modules::operand::Label;
use petgraph::algo::dominators::Dominators;
use smallvec::SmallVec;

pub trait DominatorExtension<N: Copy> {
    /// Returns true if `a` dominates `b`.
    fn dominates(&self, a: N, b: N) -> bool;

    /// Returns true if `a` is dominated by `b`.
    fn dominated_by(&self, a: N, b: N) -> bool {
        self.dominates(b, a)
    }
}

impl<N> DominatorExtension<N> for Dominators<N>
where
    N: Eq + std::hash::Hash + Copy,
{
    fn dominates(&self, a: N, b: N) -> bool {
        if a == b {
            return true; // A node always dominates itself
        }

        if let Some(mut iterator) = self.dominators(b) {
            iterator.any(|n| n == a)
        } else {
            false
        }
    }
}

/// A simple representation of a "natural loop" in a CFG.
///
/// A natural loop is a loop that has a single entry point (the header)
///
/// Definition of a natural loop (adapted from LLVM):
///  - A subgraph of the CFG such that
///    (1) The subgraph forms a SCC (Strongly Connected Component) of the CFG (i.e. there is a path
///        from any node of the subgraph to any other node of the subgraph).
///    (2) All edges from outside the subgraph **INTO** the subgraph goes through a single node,
///        called the header of the loop.
///    (3) The loop is the maximum subset with these properties. That is, no additional nodes
///        from the CFG can be added to the loop while remaining a SCC and having the same header.
///
/// Concequently,
///  - A natural loop has a single entry point (the header).
///  - Loops can be nested forming a hierarchy of loops (forest like structure).
///  - Region of the program that are unreachable do not belong to any loop (i.e. we don't care about them).
///
#[derive(Clone, Debug)]
pub struct NaturalLoop {
    pub header: Label,
    pub body: BTreeSet<Label>,
}

#[derive(Component, Clone)]
pub struct LoopAnalysis {
    pub dominator: Dominators<Label>,
    pub post_dominator: Dominators<Label>,

    pub unreachable_nodes: BTreeSet<Label>,
}

impl LoopAnalysis {
    /// A special label used as a sink node for the EXIT label in the CFG when computing post-dominators.
    pub const VIRTUAL_EXIT_LABEL: Label = Label::RESERVED_1;
}

#[derive(Default)]
pub struct DeriveDominatorLocal {
    stack: Vec<Label>,
    temp_edges: SmallVec<Label, 12>,
}

pub fn derive_dominators_analysis_system(
    query: Query<(Entity, &FunctionComponent, &ControlFlowGraph), Without<LoopAnalysis>>,
    module_query: Query<&Children, With<crate::instance::core::ModuleComponent>>,
    module_res: Res<CurrentModuleFilter>,
    logger: Res<LoggerStateRes>,
    mut commands: Commands,
    mut local_state: Local<DeriveDominatorLocal>,
) {
    // Find the module corresponding to our query
    let module_children = match module_query.get(module_res.entity) {
        Ok(children) => children,
        Err(_) => {
            hyerror!(
                logger;
                "Failed to find module for dominators analysis derivation. Skipping dominators analysis derivation."
            );
            return;
        }
    };

    for (entity, func, cfg) in query.iter_many(module_children) {
        // Cloning the graph is required to add the `sink` node to the graph.
        let mut graph = cfg.cfg.clone();
        graph.add_node(LoopAnalysis::VIRTUAL_EXIT_LABEL); // Add a sink node for the EXIT label
        local_state.temp_edges.clear();

        for node in graph.nodes() {
            let terminator = &func.body[&node].terminator;
            if terminator.is_terminating() {
                local_state.temp_edges.push(node);
            }
        }
        for node in &local_state.temp_edges {
            graph.add_edge(*node, LoopAnalysis::VIRTUAL_EXIT_LABEL, None);
        }

        // Build the LoopAnalysis for this function
        let reversed_graph = petgraph::visit::Reversed(&graph);

        let dominator = petgraph::algo::dominators::simple_fast(&graph, Label::ENTRY);
        let post_dominator = petgraph::algo::dominators::simple_fast(
            &reversed_graph,
            LoopAnalysis::VIRTUAL_EXIT_LABEL,
        );

        // We use an approach akin to the one used in LLVM to find "standard loops". This
        // approach does **NOT** generalizes to all type of cycles within the CFG, but it is
        // often sufficient.
        // We find/define loop as follow
        // 1. Find a **loop-header**, an element such that for all path going **in** the loop,
        //    the path go through the loop-header (i.e. forall node of the loop, the loop-header dominates it).
        //
        //    To find this, we look for a back-edge in the CFG (i.e. an edge from a node to one of its dominators).
        let mut unreachable_nodes = BTreeSet::new();
        let mut loops = Vec::new();

        for node in graph.nodes() {
            // Check whether there is a path from the ENTRY to the current node
            if dominator.dominators(node).is_none() {
                unreachable_nodes.insert(node);
                continue; // Loop that are unreachable are not interesting anyway, just skip them.
            }

            // Determine if the current node is a back-edge
            local_state.temp_edges.clear();
            for other_node in graph.neighbors(node) {
                // If the neighbor is a dominator of the node, then we have a back-edge.
                if dominator.dominates(other_node, node) {
                    local_state.temp_edges.push(other_node);
                }
            }

            // This node has a back-edge
            while let Some(loop_header) = local_state.temp_edges.pop() {
                // If a [`NaturalLoop`] already exists with the same header, skip back-edge
                if loops.iter().any(|l: &NaturalLoop| l.header == loop_header) {
                    continue;
                }

                // We have a back-edge, iterate over all the nodes that are in the loop
                // (i.e. all the nodes that are dominated by the header and that can reach
                //  the back-edge)
                let mut loop_body = BTreeSet::new();

                // Explore the graph from the loop_header, and find all nodes that are
                // (1) dominated by the loop_header and
                // (2) can reach a back-edge to the loop_header
                local_state.stack.clear();
                local_state.stack.push(loop_header);

                while let Some(node) = local_state.stack.pop() {
                    if loop_body.contains(&node) {
                        continue;
                    }

                    // Check whether the node is dominated by the loop header
                    if !dominator.dominates(loop_header, node) {
                        continue;
                    }

                    // Check whether the node can reach the back-edge
                    if !petgraph::algo::has_path_connecting(&graph, node, loop_header, None) {
                        continue;
                    }

                    // Push all the neighbors of the node to the stack
                    for neighbor in graph.neighbors(node) {
                        local_state.stack.push(neighbor);
                    }
                    loop_body.insert(node);
                }

                // Add the loop
                loops.push(NaturalLoop {
                    header: loop_header,
                    body: loop_body,
                });
            }
        }

        // Insert the LoopAnalysis as a component
        commands.entity(entity).insert(LoopAnalysis {
            dominator,
            post_dominator,
            unreachable_nodes,
        });
    }
}

pub struct DeriveLoopAnalysisPlugin;
impl Plugin for DeriveLoopAnalysisPlugin {
    fn init(
        &mut self,
        instance: &mut crate::instance::Instance,
        _ext: Option<&mut crate::ext::ExtList>,
    ) -> crate::HyResult<()> {
        instance.add_systems(
            PostUpdate,
            derive_dominators_analysis_system.after(derive_cfg_system),
        );
        Ok(())
    }
}
