use std::sync::Arc;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use hyinstr::{
    consts::AnyConst,
    modules::{
        self, BasicBlock, Function, Module,
        instructions::{
            HyInstr, Instruction,
            int::{IAdd, ICmp, ICmpVariant, OverflowSignednessPolicy},
            misc::{Invoke, Phi},
        },
        operand::{Label, Name, Operand},
        parser::{extend_module_from_path, extend_module_from_string},
        symbol::{FunctionPointer, FunctionPointerType},
        terminator::{Branch, HyTerminator, Jump, Ret},
    },
    types::{
        TypeRegistry, Typeref,
        primary::{IType, WType},
    },
    utils::Error,
};
use uuid::Uuid;

fn registry() -> TypeRegistry {
    TypeRegistry::new([0; 6])
}

fn i1(reg: &TypeRegistry) -> Typeref {
    reg.search_or_insert(IType::I1.into())
}

fn i32(reg: &TypeRegistry) -> Typeref {
    reg.search_or_insert(IType::I32.into())
}

fn block(label: Label, instructions: Vec<HyInstr>, terminator: HyTerminator) -> BasicBlock {
    BasicBlock {
        label,
        instructions,
        terminator,
    }
}

fn function(
    name: &str,
    params: Vec<(Name, Typeref)>,
    blocks: Vec<BasicBlock>,
    return_type: Option<Typeref>,
    wildcard_types: BTreeSet<WType>,
    meta_function: bool,
) -> Function {
    Function {
        name: Some(name.to_string()),
        params,
        return_type,
        body: blocks.into_iter().map(|bb| (bb.label, bb)).collect(),
        wildcard_types,
        meta_function,
        ..Default::default()
    }
}

fn simple_ok_function(reg: &TypeRegistry) -> Function {
    let ty = i32(reg);
    let add = HyInstr::from(IAdd {
        dest: Name(1),
        ty,
        lhs: Operand::Reg(Name(0)),
        rhs: Operand::Imm(1u32.into()),
        variant: OverflowSignednessPolicy::Wrap,
    });
    let entry = block(
        Label::NIL,
        vec![add],
        HyTerminator::from(Ret {
            value: Some(Operand::Reg(Name(1))),
        }),
    );

    function(
        "ok",
        vec![(Name(0), ty)],
        vec![entry],
        Some(ty),
        BTreeSet::new(),
        false,
    )
}

#[test]
fn function_verify_simple_passes() {
    let reg = registry();
    let func = simple_ok_function(&reg);
    assert!(func.verify().is_ok());
}

#[test]
fn function_normalize_ssa_relabels_all_uses() {
    let reg = registry();
    let ty = i32(&reg);
    let mut func = function(
        "normalize",
        vec![(Name(10), ty), (Name(12), ty)],
        vec![{
            let add = HyInstr::from(IAdd {
                dest: Name(20),
                ty,
                lhs: Operand::Reg(Name(10)),
                rhs: Operand::Reg(Name(12)),
                variant: OverflowSignednessPolicy::Wrap,
            });
            block(
                Label::NIL,
                vec![add],
                HyTerminator::from(Ret {
                    value: Some(Operand::Reg(Name(20))),
                }),
            )
        }],
        Some(ty),
        BTreeSet::new(),
        false,
    );

    func.normalize_ssa();

    let param_names: Vec<Name> = func.params.iter().map(|(n, _)| *n).collect();
    assert_eq!(param_names, vec![Name(0), Name(1)]);

    let (instr, _) = func.iter().next().unwrap();
    let dest = instr.destination().unwrap();
    assert_eq!(dest, Name(2));

    let deps: Vec<Name> = instr.dependencies().collect();
    assert_eq!(deps, vec![Name(0), Name(1)]);

    if let HyTerminator::Ret(ret) = &func.body[&Label::NIL].terminator {
        assert_eq!(ret.value, Some(Operand::Reg(Name(2))));
    } else {
        panic!("ret missing");
    }
}

#[test]
fn function_verify_rejects_duplicate_ssa() {
    let reg = registry();
    let ty = i32(&reg);
    let add1 = HyInstr::from(IAdd {
        dest: Name(1),
        ty,
        lhs: Operand::Reg(Name(0)),
        rhs: Operand::Imm(1u32.into()),
        variant: OverflowSignednessPolicy::Wrap,
    });
    let add2 = HyInstr::from(IAdd {
        dest: Name(1),
        ty,
        lhs: Operand::Reg(Name(0)),
        rhs: Operand::Imm(2u32.into()),
        variant: OverflowSignednessPolicy::Wrap,
    });
    let func = function(
        "dup",
        vec![(Name(0), ty)],
        vec![block(
            Label::NIL,
            vec![add1, add2],
            HyTerminator::from(Ret {
                value: Some(Operand::Reg(Name(1))),
            }),
        )],
        Some(ty),
        BTreeSet::new(),
        false,
    );

    let err = func.verify().unwrap_err();
    assert!(matches!(err, Error::DuplicateSSAName { duplicate } if duplicate == Name(1)));
}

#[test]
fn function_verify_requires_entry_block() {
    let reg = registry();
    let ty = i32(&reg);
    let func = function(
        "no_entry",
        vec![(Name(0), ty)],
        vec![block(
            Label(1),
            vec![],
            HyTerminator::from(Jump { target: Label(1) }),
        )],
        Some(ty),
        BTreeSet::new(),
        false,
    );

    assert!(matches!(func.verify(), Err(Error::MissingEntryBlock)));
}

#[test]
fn function_verify_detects_undefined_operand_and_block() {
    let reg = registry();
    let ty = i32(&reg);
    let add = HyInstr::from(IAdd {
        dest: Name(1),
        ty,
        lhs: Operand::Reg(Name(0)),
        rhs: Operand::Reg(Name(99)), // undefined
        variant: OverflowSignednessPolicy::Wrap,
    });
    let func = function(
        "undef",
        vec![(Name(0), ty)],
        vec![block(
            Label::NIL,
            vec![add],
            HyTerminator::from(Branch {
                cond: Operand::Reg(Name(0)),
                target_true: Label(1), // missing block
                target_false: Label::NIL,
            }),
        )],
        Some(ty),
        BTreeSet::new(),
        false,
    );

    let err = func.verify().unwrap_err();
    assert!(matches!(
        err,
        Error::UndefinedSSAName { .. } | Error::UndefinedBasicBlock { .. }
    ));
}

#[test]
fn function_verify_rejects_phi_not_first() {
    let reg = registry();
    let ty = i32(&reg);
    let phi = HyInstr::from(Phi {
        dest: Name(2),
        ty,
        values: vec![(Operand::Reg(Name(1)), Label::NIL)],
    });
    let add = HyInstr::from(IAdd {
        dest: Name(1),
        ty,
        lhs: Operand::Reg(Name(0)),
        rhs: Operand::Imm(1u32.into()),
        variant: OverflowSignednessPolicy::Wrap,
    });
    let func = function(
        "phi_order",
        vec![(Name(0), ty)],
        vec![block(
            Label::NIL,
            vec![add, phi],
            HyTerminator::from(Ret {
                value: Some(Operand::Reg(Name(2))),
            }),
        )],
        Some(ty),
        BTreeSet::new(),
        false,
    );

    assert!(
        matches!(func.verify(), Err(Error::PhiNotFirstInstruction { block }) if block == Label::NIL)
    );
}

#[test]
fn function_verify_rejects_meta_elements_in_non_meta_function() {
    let reg = registry();
    let ty = i32(&reg);
    let meta_op_instr = HyInstr::from(IAdd {
        dest: Name(1),
        ty,
        lhs: Operand::Imm(0u32.into()),
        rhs: Operand::Imm(0u32.into()),
        variant: OverflowSignednessPolicy::Wrap,
    });
    let meta_instr = HyInstr::MetaAssert(modules::instructions::meta::MetaAssert {
        condition: Operand::Reg(Name(1)),
    });
    let func = function(
        "meta",
        vec![(Name(0), ty)],
        vec![block(
            Label::NIL,
            vec![meta_op_instr, meta_instr],
            HyTerminator::from(Ret { value: None }),
        )],
        None,
        BTreeSet::new(),
        false,
    );

    let err = func.verify().unwrap_err();
    assert!(matches!(
        err,
        Error::MetaOperandNotAllowed | Error::MetaInstructionNotAllowed { .. }
    ));
}

#[test]
fn function_verify_checks_wildcard_soundness() {
    let _reg = registry();
    let wildcard = Typeref::new_wildcard(7);
    let instr = HyInstr::from(IAdd {
        dest: Name(1),
        ty: wildcard,
        lhs: Operand::Reg(Name(0)),
        rhs: Operand::Imm(1u32.into()),
        variant: OverflowSignednessPolicy::Wrap,
    });
    let func = function(
        "wildcard",
        vec![(Name(0), wildcard)],
        vec![block(
            Label::NIL,
            vec![instr],
            HyTerminator::from(Ret {
                value: Some(Operand::Reg(Name(1))),
            }),
        )],
        Some(wildcard),
        BTreeSet::new(), // missing wildcard declaration
        false,
    );

    assert!(matches!(
        func.verify(),
        Err(Error::UnsoundWildcardTypes { .. })
    ));
}

#[test]
fn function_verify_enforces_parameter_limit() {
    let reg = registry();
    let ty = i32(&reg);
    let params: Vec<_> = (0..=Function::MAX_PARAMS_PER_FUNC as u32)
        .map(|i| (Name(i), ty))
        .collect();
    let func = function(
        "too_many_params",
        params,
        vec![block(
            Label::NIL,
            vec![],
            HyTerminator::from(Ret { value: None }),
        )],
        None,
        BTreeSet::new(),
        false,
    );

    assert!(matches!(
        func.verify(),
        Err(Error::FunctionTooManyArguments { .. })
    ));
}

#[test]
fn function_analysis_helpers_produce_expected_graphs() {
    let reg = registry();
    let ty = i32(&reg);

    // entry: branch to l1 or l2
    let entry_block = block(
        Label::NIL,
        vec![HyInstr::from(ICmp {
            dest: Name(1),
            ty: i1(&reg),
            lhs: Operand::Reg(Name(0)),
            rhs: Operand::Imm(0u32.into()),
            variant: ICmpVariant::Eq,
        })],
        HyTerminator::from(Branch {
            cond: Operand::Reg(Name(1)),
            target_true: Label(1),
            target_false: Label(2),
        }),
    );
    // l1: jump l3
    let l1 = block(
        Label(1),
        vec![],
        HyTerminator::from(Jump { target: Label(3) }),
    );
    // l2: jump l3
    let l2 = block(
        Label(2),
        vec![],
        HyTerminator::from(Jump { target: Label(3) }),
    );
    // l3: ret %0
    let l3 = block(
        Label(3),
        vec![HyInstr::from(IAdd {
            dest: Name(2),
            ty,
            lhs: Operand::Reg(Name(0)),
            rhs: Operand::Imm(1u32.into()),
            variant: OverflowSignednessPolicy::Wrap,
        })],
        HyTerminator::from(Ret {
            value: Some(Operand::Reg(Name(2))),
        }),
    );

    let func = function(
        "flow",
        vec![(Name(0), ty)],
        vec![entry_block, l1, l2, l3],
        Some(ty),
        BTreeSet::new(),
        false,
    );
    func.verify().unwrap();
    let func = Arc::new(func);

    let cfg = func.derive_function_flow();
    assert!(cfg.contains_node(Label::NIL));
    assert_eq!(cfg.edge_count(), 4);
    assert!(cfg.edge_weight(Label::NIL, Label(1)).is_some());
    assert!(cfg.edge_weight(Label::NIL, Label(2)).is_some());
    assert!(cfg.edge_weight(Label(1), Label(3)).is_some());
    assert!(cfg.edge_weight(Label(2), Label(3)).is_some());

    let dest_map = func.derive_dest_map();
    assert_eq!(dest_map.get(&Name(1)).unwrap().block, Label::NIL);
    assert_eq!(dest_map.get(&Name(2)).unwrap().block, Label(3));

    let phis: Vec<_> = func
        .gather_instructions_by_predicate(|instr| instr.is_simple())
        .into_iter()
        .collect();
    assert!(phis.len() >= 2);

    let ctx = func.analyze();
    assert_eq!(ctx.cfg.node_count(), cfg.node_count());
    assert_eq!(ctx.dest_map.len(), dest_map.len());
}

#[test]
fn module_verify_func_fails_on_missing_internal_or_external() {
    let reg = registry();
    let ty = i32(&reg);

    // Caller referencing missing internal function
    let missing_internal = FunctionPointer::Internal(Uuid::new_v4());
    let call_instr = HyInstr::from(Invoke {
        function: Operand::Imm(AnyConst::FuncPtr(missing_internal.clone())),
        args: vec![Operand::Reg(Name(0))],
        dest: Some(Name(1)),
        ty: Some(ty),
        cconv: None,
    });
    let caller = function(
        "caller",
        vec![(Name(0), ty)],
        vec![block(
            Label::NIL,
            vec![call_instr],
            HyTerminator::from(Ret {
                value: Some(Operand::Reg(Name(1))),
            }),
        )],
        Some(ty),
        BTreeSet::new(),
        false,
    );
    let module = Module::default();
    let err = module.verify_func(&caller).unwrap_err();
    assert!(matches!(err, Error::UndefinedInternalFunction { .. }));

    // Caller referencing missing external function
    let module = Module::default();
    let missing_external = FunctionPointer::External(Uuid::new_v4());
    let call_instr = HyInstr::from(Invoke {
        function: Operand::Imm(AnyConst::FuncPtr(missing_external.clone())),
        args: vec![],
        dest: None,
        ty: None,
        cconv: None,
    });
    let caller = function(
        "caller2",
        vec![],
        vec![block(
            Label::NIL,
            vec![call_instr],
            HyTerminator::from(Ret { value: None }),
        )],
        None,
        BTreeSet::new(),
        false,
    );
    let err = module.verify_func(&caller).unwrap_err();
    assert!(matches!(err, Error::UndefinedExternalFunction { .. }));
}

#[test]
fn module_verify_succeeds_when_functions_resolved() {
    let reg = registry();
    let ty = i32(&reg);

    let mut module = Module::default();

    // Callee definition
    let callee_uuid = Uuid::new_v4();
    let callee = Function {
        uuid: callee_uuid,
        name: Some("callee".into()),
        params: vec![(Name(0), ty)],
        return_type: Some(ty),
        body: BTreeMap::from([(
            Label::NIL,
            block(
                Label::NIL,
                vec![HyInstr::from(IAdd {
                    dest: Name(1),
                    ty,
                    lhs: Operand::Reg(Name(0)),
                    rhs: Operand::Imm(1u32.into()),
                    variant: OverflowSignednessPolicy::Wrap,
                })],
                HyTerminator::from(Ret {
                    value: Some(Operand::Reg(Name(1))),
                }),
            ),
        )]),
        ..Default::default()
    };
    module.functions.insert(callee_uuid, Arc::new(callee));

    // Caller referencing callee
    let call_instr = HyInstr::from(Invoke {
        function: Operand::Imm(AnyConst::FuncPtr(FunctionPointer::Internal(callee_uuid))),
        args: vec![Operand::Imm(1u32.into())],
        dest: Some(Name(1)),
        ty: Some(ty),
        cconv: None,
    });
    let caller = function(
        "caller",
        vec![(Name(0), ty)],
        vec![block(
            Label::NIL,
            vec![call_instr],
            HyTerminator::from(Ret {
                value: Some(Operand::Reg(Name(1))),
            }),
        )],
        Some(ty),
        BTreeSet::new(),
        false,
    );
    module.functions.insert(caller.uuid, Arc::new(caller));

    module.verify().unwrap();
}

#[test]
fn parser_simple_round_trip_from_string() {
    let reg = registry();
    let mut module = Module::default();

    let source = r#"
        define i32 add_one(%x: i32) {
        entry:
            %y: i32 = iadd.wrap %x, i32 1
            ret %y
        }
    "#;

    extend_module_from_string(&mut module, &reg, source).unwrap();

    let uuid = module
        .find_internal_function_uuid_by_name("add_one")
        .expect("function should exist");
    let func = module.get_internal_function_by_uuid(uuid).unwrap();
    assert_eq!(func.params.len(), 1);
    assert_eq!(func.params[0].0, Name(0));
    let first_instr = &func.body[&Label::NIL].instructions[0];
    assert_eq!(first_instr.destination(), Some(Name(1)));
}

#[test]
fn parser_handles_imports_with_extend_module_from_path() {
    let reg = registry();
    let temp_dir = std::env::temp_dir().join(format!("hyinstr_tests_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();

    let dep_content = r#"
        define i32 inc(%x: i32) {
        entry:
            %y: i32 = iadd.wrap %x, i32 1
            ret %y
        }
    "#;
    let dep_path = temp_dir.join("dep.func");
    fs::write(&dep_path, dep_content).unwrap();

    let main_content = r#"
        import "dep.func"
        define void main() {
        entry:
            ret void
        }
    "#;
    let main_path = temp_dir.join("main.func");
    fs::write(&main_path, main_content).unwrap();

    let mut module = Module::default();
    extend_module_from_path(&mut module, &reg, &main_path).unwrap();

    assert!(module.find_internal_function_uuid_by_name("inc").is_some());
    assert!(module.find_internal_function_uuid_by_name("main").is_some());

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn parser_extended_factorial_example_resolves_calls() {
    let reg = registry();
    let mut module = Module::default();
    let ir = r#"
define i32 factorial ( %n: i32 ) {
entry:
   %cmp1: i1 = icmp.eq %n, i32 0
   branch %cmp1, return_result, recurse

recurse:
   %n_minus_1: i32 = isub.wrap %n, i32 1
   %recursive_result: i32 = invoke ptr factorial, %n_minus_1
   %result2: i32 = imul.usat  %n, %recursive_result
   %result: i32 = imul.wrap %n, %recursive_result
   jump return_result

return_result:
   %final_result: i32 = phi [ %result2, recurse ], [ i32 1, entry ]
   ret %final_result
}

; Free variable n <=> forall n
define void !factorial_test_a (%n: i32) {
entry:
    %n_less_1: i32 = isub.wrap %n, i32 1
    %n_greater_0: i1 = icmp.ugt %n, i32 0
    !assume %n_greater_0
    %fact_n: i32 = invoke ptr factorial, %n
    %fact_n_minus_0: i32 = invoke ptr factorial, %n_less_1

    ; This meta-function is the properties that fact(n) = n * fact(n - 1) for n > 0
    %prod: i32 = imul.wrap %n, %fact_n_minus_0
    %eq: i1 = icmp.eq %fact_n, %prod
    !assert %eq

    ret void
}

define void !factorial_test_b () {
entry:
    %fact_0: i32 = invoke ptr factorial, i32 0
    %fact_1: i32 = invoke ptr factorial, i32 1
    %eq0: i1 = icmp.eq %fact_0, i32 1
    %eq1: i1 = icmp.eq %fact_1, i32 1
    %eq_final: i1 = and %eq0, %eq1
    !assert %eq_final
    ret void
}
"#;

    extend_module_from_string(&mut module, &reg, ir).unwrap();

    let factorial_uuid = module
        .find_internal_function_uuid_by_name("factorial")
        .unwrap();
    let test_a_uuid = module
        .find_internal_function_uuid_by_name("factorial_test_a")
        .unwrap();
    let test_b_uuid = module
        .find_internal_function_uuid_by_name("factorial_test_b")
        .unwrap();

    assert_ne!(factorial_uuid, test_a_uuid);
    assert_ne!(factorial_uuid, test_b_uuid);

    let test_a = module.get_internal_function_by_uuid(test_a_uuid).unwrap();
    assert!(test_a.meta_function);

    // Ensure invokes inside test_a point to the parsed factorial uuid
    let mut seen_call = false;
    for (instr, _) in test_a.iter() {
        if let HyInstr::Invoke(inv) = instr {
            if let Operand::Imm(AnyConst::FuncPtr(FunctionPointer::Internal(uuid))) = inv.function {
                assert_eq!(uuid, factorial_uuid);
                seen_call = true;
            }
        }
    }
    assert!(seen_call);
}

#[test]
fn parser_parses_meta_forall_zero_arity_and_bool_type() {
    let reg = registry();
    let mut module = Module::default();
    let src = r#"
        define void !quant() {
        entry:
            %x: i1 = !forall
            !assume %x
            ret void
        }
    "#;

    extend_module_from_string(&mut module, &reg, src).unwrap();
    let uuid = module
        .find_internal_function_uuid_by_name("quant")
        .expect("function should exist");
    let func = module.get_internal_function_by_uuid(uuid).unwrap();
    assert!(func.meta_function);
    let first = &func.body[&Label::NIL].instructions[0];
    if let HyInstr::MetaForall(mf) = first {
        use hyinstr::types::primary::IType;
        let ty = reg.search_or_insert(IType::I1.into());
        assert_eq!(mf.destination(), Some(Name(0))); // normalized to first SSA
        assert_eq!(mf.destination_type(), Some(ty));
    } else {
        panic!("expected MetaForall as first instruction");
    }
}

#[test]
fn meta_analysis_stat_termination_scope_flags() {
    use hyinstr::analysis::{AnalysisStatistic, TerminationScope};

    let reg = registry();
    let ty = i32(&reg);

    let instr = HyInstr::from(modules::instructions::meta::MetaAnalysisStat {
        dest: Name(1),
        ty,
        statistic: AnalysisStatistic::TerminationBehavior(TerminationScope::FunctionExit),
    });

    assert!(instr.is_meta_instruction());
    assert!(instr.is_simple());
}

#[test]
fn parser_reports_unresolved_external_function() {
    let reg = registry();
    let mut module = Module::default();

    // Call to an external function without declaring it should fail
    let source = r#"
        define void caller() {
        entry:
            %r: i32 = invoke ptr external printf, i32 0
            ret void
        }
    "#;

    let err = extend_module_from_string(&mut module, &reg, source).unwrap_err();
    assert!(
        matches!(err, Error::UnresolvedFunction { func_type, .. } if func_type == FunctionPointerType::External)
    );
}

#[test]
fn parser_parses_meta_analysis_stat_termination_variant() {
    use hyinstr::analysis::{AnalysisStatistic, TerminationScope};

    let reg = registry();
    let mut module = Module::default();
    let src = r#"
        define void !ana() {
        entry:
            %x: i32 = !analysis.term.funcexit
            ret void
        }
    "#;

    extend_module_from_string(&mut module, &reg, src).unwrap();
    let uuid = module
        .find_internal_function_uuid_by_name("ana")
        .expect("function should exist");
    let func = module.get_internal_function_by_uuid(uuid).unwrap();
    assert!(func.meta_function);
    let first = &func.body[&Label::NIL].instructions[0];
    if let HyInstr::MetaAnalysisStat(mas) = first {
        let ty = i32(&reg);
        assert_eq!(mas.destination(), Some(Name(0)));
        assert_eq!(mas.destination_type(), Some(ty));
        assert_eq!(
            mas.statistic,
            AnalysisStatistic::TerminationBehavior(TerminationScope::FunctionExit)
        );
    } else {
        panic!("expected MetaAnalysisStat as first instruction");
    }
}

#[test]
fn parser_parses_meta_analysis_stat_instruction_count_operand() {
    use hyinstr::analysis::AnalysisStatistic;
    use hyinstr::modules::instructions::InstructionFlags;

    let reg = registry();
    let mut module = Module::default();
    let src = r#"
        define void !ana2() {
        entry:
            %x: i32 = !analysis.icnt i32 0x400
            ret void
        }
    "#;

    extend_module_from_string(&mut module, &reg, src).unwrap();
    let uuid = module
        .find_internal_function_uuid_by_name("ana2")
        .expect("function should exist");
    let func = module.get_internal_function_by_uuid(uuid).unwrap();
    assert!(func.meta_function);
    let first = &func.body[&Label::NIL].instructions[0];
    if let HyInstr::MetaAnalysisStat(mas) = first {
        let ty = i32(&reg);
        assert_eq!(mas.destination(), Some(Name(0)));
        assert_eq!(mas.destination_type(), Some(ty));
        assert_eq!(
            mas.statistic,
            AnalysisStatistic::InstructionCount(InstructionFlags::MEMORY)
        );
    } else {
        panic!("expected MetaAnalysisStat as first instruction");
    }
}
