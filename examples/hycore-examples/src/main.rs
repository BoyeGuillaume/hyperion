use hycore::tests_utils::example_a;
use hyinstr::{modules::Module, types::TypeRegistry};

fn main() {
    let mut type_registry = TypeRegistry::new([0; 6]);
    let func = example_a(&mut type_registry);

    // Create a module to hold the function
    let mod_a = Module {
        functions: vec![func.clone()]
            .into_iter()
            .map(|f| (f.uuid, f))
            .collect(),
        external_functions: Default::default(),
    };

    // Validate the module
    match mod_a.check_ssa() {
        Ok(_) => println!("Module is valid SSA."),
        Err(e) => eprintln!("Module SSA validation error: {}", e),
    }

    // Display the control flow of the function
    let cfg = func.derive_function_flow();
    println!("Control Flow Graph of factorial function:");
    for edge in cfg.all_edges() {
        match edge.2 {
            Some(op) => println!("  {} --[{:?}]-> {}", edge.0, op, edge.1),
            None => println!("  {} --> {}", edge.0, edge.1),
        }
    }

    // Display each block
    for function in mod_a.functions.values() {
        println!(
            "declare {} {} %{} ({})",
            function
                .return_type
                .map(|ty| type_registry.fmt(ty).to_string())
                .unwrap_or("void".to_string()),
            function.uuid,
            function.name.as_deref().unwrap_or("<unnamed>"),
            function
                .params
                .iter()
                .map(|(name, ty)| format!("%{}: {}", name, type_registry.fmt(*ty)))
                .collect::<Vec<_>>()
                .join(", ")
        );

        for (uuid, block) in function.body.iter() {
            println!("  {}: ", uuid);
            if !block.instructions.is_empty() {
                println!(
                    "{}",
                    block
                        .instructions
                        .iter()
                        .map(|instr| format!("   {}", instr.fmt(&type_registry, Some(&mod_a))))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            println!("   {}", block.terminator.fmt(Some(&mod_a)));
        }
    }
}
