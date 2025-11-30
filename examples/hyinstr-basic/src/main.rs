use hyinstr::{
    consts::AnyConst,
    modules::{
        BasicBlock, CallingConvention, Module,
        int::{ICmp, ICmpOp, IMul, ISub, IntegerSignedness, OverflowPolicy},
        misc::Invoke,
        operand::{Label, Operand},
        symbol::FunctionPointer,
        terminator::Ret,
    },
    types::{TypeRegistry, primary::IType},
};
use uuid::Uuid;

fn main() {
    // Create a simple script that uses hyinstr to define a function (factorial)
    // Psoeudo code:
    //   fn factorial(n: i32) -> i32 {
    //       if n <= 1 {
    //           return 1;
    //       } else {
    //           return n * factorial(n - 1);
    //       }
    //   }
    //
    //  %1 = icmp sle i32 %n, 1
    //  br i1 %1, label %base_case, label %recurse
    //
    // base_case:
    //  ret i32 1
    // recurse_a:
    //  %2 = sub i32 %n, 1
    //  %3 = call i32 @factorial(i32 %2)
    // recurse_b:
    //  %4 = mul i32 %n, %3
    //  ret i32 %4
    let type_registry = TypeRegistry::new([0; 6]);
    let i32_ty = type_registry.search_or_insert(IType::I32.into());
    let factorial_func_uuid = Uuid::new_v4();

    let block_base_case = BasicBlock {
        label: Label(1),
        instructions: vec![],
        terminator: Ret {
            value: Some(Operand::Imm(1u32.into())),
        }
        .into(),
    };

    let block_recurse_a = BasicBlock {
        label: Label(2),
        instructions: vec![
            ISub {
                dest: 2,
                ty: i32_ty,
                lhs: Operand::Reg(0),
                rhs: Operand::Imm(1u32.into()),
                overflow: OverflowPolicy::Panic,
                signedness: IntegerSignedness::Unsigned,
            }
            .into(),
            Invoke {
                function: Operand::Imm(AnyConst::FuncPtr(FunctionPointer::Internal(
                    factorial_func_uuid,
                ))),
                args: vec![Operand::Reg(2)],
                dest: Some(5),
                ty: Some(i32_ty),
                cconv: None,
            }
            .into(),
            IMul {
                dest: 8,
                ty: i32_ty,
                lhs: Operand::Reg(0),
                rhs: Operand::Reg(5),
                overflow: OverflowPolicy::Panic,
                signedness: IntegerSignedness::Unsigned,
            }
            .into(),
        ],
        terminator: Ret {
            value: Some(Operand::Reg(8)),
        }
        .into(),
    };

    let block_entry = BasicBlock {
        label: Label(0),
        instructions: vec![
            ICmp {
                dest: 1,
                ty: i32_ty,
                lhs: Operand::Reg(0),
                rhs: Operand::Imm(1u32.into()),
                op: ICmpOp::Sle,
            }
            .into(),
        ],
        terminator: hyinstr::modules::terminator::CBranch {
            cond: Operand::Reg(1),
            target_true: block_base_case.label(),
            target_false: block_recurse_a.label(),
        }
        .into(),
    };

    let mut factorial_function = hyinstr::modules::Function {
        uuid: factorial_func_uuid,
        name: Some("factorial".to_string()),
        cconv: Some(CallingConvention::C),
        params: vec![(0, i32_ty)],
        return_type: Some(i32_ty),
        body: vec![block_entry, block_base_case, block_recurse_a]
            .into_iter()
            .map(|bb| (bb.label, bb))
            .collect(),
        visibility: Some(hyinstr::modules::Visibility::Default),
        wildcard_types: Default::default(),
        meta_function: false,
    };

    factorial_function.normalize_ssa();

    // Create a module to hold the function
    let mod_a = Module {
        functions: vec![factorial_function.clone()]
            .into_iter()
            .map(|f| (f.uuid, f))
            .collect(),
        external_functions: Default::default(),
    };

    // Validate the module
    match mod_a.verify() {
        Ok(_) => println!("Module is valid SSA."),
        Err(e) => eprintln!("Module SSA validation error: {}", e),
    }

    // Display the control flow of the function
    let cfg = factorial_function.derive_function_flow();
    println!("Control Flow Graph of factorial function:");
    for edge in cfg.all_edges() {
        match edge.2 {
            Some(op) => println!("  {} --[{:?}]-> {}", edge.0, op, edge.1),
            None => println!("  {} --> {}", edge.0, edge.1),
        }
    }

    // Display each block
    for function in mod_a.functions.values() {
        println!("{}", function.fmt(&type_registry, Some(&mod_a)));
        // println!(
        //     "declare {} {} %{} ({})",
        //     function
        //         .return_type
        //         .map(|ty| type_registry.fmt(ty).to_string())
        //         .unwrap_or("void".to_string()),
        //     function.uuid,
        //     function.name.as_deref().unwrap_or("<unnamed>"),
        //     function
        //         .params
        //         .iter()
        //         .map(|(name, ty)| format!("%{}: {}", name, type_registry.fmt(*ty)))
        //         .collect::<Vec<_>>()
        //         .join(", ")
        // );

        // for (uuid, block) in function.body.iter() {
        //     println!("  {}: ", uuid);
        //     if !block.instructions.is_empty() {
        //         println!(
        //             "{}",
        //             block
        //                 .instructions
        //                 .iter()
        //                 .map(|instr| format!("   {}", instr.fmt(&type_registry, Some(&mod_a))))
        //                 .collect::<Vec<_>>()
        //                 .join("\n")
        //         );
        //     }
        //     println!("   {}", block.terminator.fmt(Some(&mod_a)));
        // }
    }
}
