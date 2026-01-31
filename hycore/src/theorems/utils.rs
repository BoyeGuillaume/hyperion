//! Lightweight optimization passes shared by theorems tooling and provers.

use std::collections::HashMap;

use bit_set::BitSet;
use hyinstr::modules::{
    Function, InstructionRef,
    instructions::{HyInstr, HyInstrOp, Instruction},
    operand::{Label, Name},
};
use petgraph::graph::DiGraph;

use crate::utils::error::HyResult;

/// A simple simplification pass for functions
///
/// This simplification focuses on basic optimizations that do not require complex.
/// - It merges similar operands that do not have side effects together (within the same block) to reduce redundancy.
///
/// # Arguments
/// * `func` - The function to simplify
///
pub fn simple_simplify_function(func: &mut Function) -> HyResult<()> {
    #[cfg(debug_assertions)]
    func.verify()
        .expect("Function must be valid before simplifying");

    // Detect repeated patterns and simplify them here
    // Merge similar operands (e.g. if we have two `add $1, $2` instructions, we can merge them IN SAME BLOCK)
    let mut previous_operand_map: HashMap<HyInstr, Option<Name>> = HashMap::new();
    let mut remapped_name: HashMap<Name, Name> = HashMap::new();

    for (_, block) in func.body.iter_mut() {
        previous_operand_map.clear();

        // First, remap operands according to previous remappings
        block.instructions.retain(|elem| {
            // non-simple instruction are ignored
            if !elem.is_simple() {
                return true;
            }

            // Clone the element and set its destination to the same value (so we can compare)
            let mut cloned_elem = elem.clone();
            cloned_elem.set_destination(Name(0));

            match previous_operand_map.get(&cloned_elem) {
                Some(existing_name) => {
                    // Remap the result name to the existing one
                    let result = elem.destination();
                    assert!(
                        result.is_some() == existing_name.is_some(),
                        "Instruction result presence mismatch"
                    );
                    if let (Some(result_name), Some(existing_name)) = (result, existing_name) {
                        remapped_name.insert(result_name, *existing_name);
                    }

                    false
                }
                None => {
                    // Insert the new operand into the hash map
                    let result = elem.destination();
                    previous_operand_map.insert(cloned_elem, result);
                    true
                }
            }
        });
    }

    // Finally, apply remapping to all instructions
    for (_, block) in func.body.iter_mut() {
        for instr in block.instructions.iter_mut() {
            instr.remap_operands(|name| remapped_name.get(&name).copied());
        }
    }

    Ok(())
}

/// Remove unused operations from the function
///
/// This pass removes instructions that do not contribute to the final output of the function,
/// i.e., instructions whose results are never used and that do not have side effects.
///
/// # Arguments
/// * `func` - The function to process
///
pub fn remove_unused_op(func: &mut Function) -> HyResult<()> {
    #[cfg(debug_assertions)]
    func.verify()
        .expect("Function must be valid before removing unused operands");

    // Notice that in this graph, edges point in the reverse direction: from src to dst
    let mut usage_graph: DiGraph<InstructionRef, ()> = DiGraph::new();
    const SOURCE_NODE: InstructionRef = InstructionRef {
        block: Label::NIL,
        index: u32::MAX,
        reserved: 0, // not used
    };
    let source_node_index = usage_graph.add_node(SOURCE_NODE);

    // Populate usage graph
    let mut name_to_node: HashMap<Name, petgraph::graph::NodeIndex> = HashMap::new();
    let mut refs_to_node: HashMap<InstructionRef, petgraph::graph::NodeIndex> = HashMap::new();

    for (instr, instr_ref) in func.iter() {
        let node_index = usage_graph.add_node(instr_ref);
        refs_to_node.insert(instr_ref, node_index);

        if let Some(dest) = instr.destination() {
            name_to_node.insert(dest, node_index);
        }

        if matches!(
            instr.op(),
            HyInstrOp::MLoad
                | HyInstrOp::MStore
                | HyInstrOp::Invoke
                | HyInstrOp::MetaAssert
                | HyInstrOp::MetaAssume
        ) {
            usage_graph.add_edge(source_node_index, node_index, ());
        }
    }
    for block in func.body.values() {
        block.terminator.operands().for_each(|operand| {
            if let Some(name) = operand.try_as_reg_ref()
                && let Some(&src_node) = name_to_node.get(name)
            {
                usage_graph.add_edge(source_node_index, src_node, ());
            }
        });
    }

    for (instr, instr_ref) in func.iter() {
        let dst = refs_to_node[&instr_ref];

        for name in instr.operands().filter_map(|x| x.try_as_reg_ref()) {
            // Can be None if the operand register refers to an argument of the function
            if let Some(&src) = name_to_node.get(name) {
                usage_graph.add_edge(dst, src, ());
            }
        }
    }

    // Finally, run a reachability analysis from all instructions that have side effects or are return instructions
    let mut reachable_nodes = BitSet::with_capacity(usage_graph.node_count());
    let mut dfs = petgraph::visit::Dfs::new(&usage_graph, source_node_index);
    while let Some(nx) = dfs.next(&usage_graph) {
        reachable_nodes.insert(nx.index());
    }

    // Remove unreachable instructions
    for (block_label, block) in func.body.iter_mut() {
        let mut instr_index = 0;
        block.instructions.retain(|_| {
            let instr_ref = InstructionRef {
                block: *block_label,
                index: instr_index,
                reserved: 0, // not used
            };
            instr_index += 1;
            let node_index = refs_to_node[&instr_ref];
            reachable_nodes.contains(node_index.index())
        });
    }

    // Verify the function after modifications
    Ok(())
}
