use std::{panic, sync::Arc};

use hyinstr::{
    attached::AttachedFunction,
    modules::{
        Function, Module,
        instructions::{
            HyInstr, Instruction,
            int::{IAdd, IDiv, IMul, ISub, OverflowSignednessPolicy},
            mem::MLoad,
            misc::Cast,
        },
        operand::{Label, Name, Operand},
        parser::extend_module_from_string,
    },
    types::{TypeRegistry, primary::IType},
};

const CHAIN_SOURCE: &str = r#"
define i32 overlay_inputs(%x: i32) {
entry:
    %twice: i32 = iadd.wrap %x, %x
    %plus_one: i32 = iadd.wrap %twice, i32 1
    ret %plus_one
}
"#;

fn build_chain_function() -> (Arc<Function>, TypeRegistry) {
    let mut module = Module::default();
    let registry = TypeRegistry::new([0; 6]);
    extend_module_from_string(&mut module, &registry, CHAIN_SOURCE)
        .expect("failed to parse sample function");

    (
        module
            .functions
            .values()
            .next()
            .cloned()
            .expect("sample function present"),
        registry,
    )
}

#[test]
fn attached_function_initializes_counters_from_target() {
    let (function, _) = build_chain_function();
    let mut attached = AttachedFunction::new(Arc::clone(&function));

    let expected_label = function.next_available_label();
    let expected_name = function.next_available_name();

    assert_eq!(attached.next_available_label(), expected_label);
    assert_eq!(
        attached.next_available_label(),
        Label(expected_label.0 + 1),
        "attached label counter should keep advancing"
    );

    assert_eq!(attached.next_available_name(), expected_name);
    assert_eq!(
        attached.next_available_name(),
        Name(expected_name.0 + 1),
        "attached name counter should keep advancing"
    );
}

#[test]
fn attached_function_pushes_and_resolves_overlay_instructions() {
    let (function, type_registry) = build_chain_function();
    let mut attached = AttachedFunction::new(Arc::clone(&function));
    let entry = function
        .body
        .get(&Label::NIL)
        .expect("entry block should exist");

    let op_a = Operand::Reg(
        entry.instructions[0]
            .destination()
            .expect("first instruction should define a value"),
    );
    let op_b = Operand::Reg(
        entry.instructions[1]
            .destination()
            .expect("second instruction should define a value"),
    );

    let i32_ty = type_registry.search_or_insert(IType::I32.into());

    let overlay_dest = attached.next_available_name();
    let new_op = IDiv {
        dest: overlay_dest,
        ty: i32_ty,
        lhs: op_a.clone(),
        rhs: op_b.clone(),
        signedness: hyinstr::modules::instructions::int::IntegerSignedness::Unsigned,
    };

    let (_, overlay_ref) = attached.push(Label::NIL, new_op.clone().into());

    assert_ne!(overlay_ref.reserved, 0);
    assert_eq!(attached.find_by_dest(&overlay_dest), Some(overlay_ref));
    assert_eq!(attached.get(overlay_ref).cloned(), Some(new_op.into()));

    let begin_dest = attached.next_available_name();
    let new_op_2 = ISub {
        dest: begin_dest,
        ty: i32_ty,
        lhs: op_b.clone(),
        rhs: op_a.clone(),
        variant: OverflowSignednessPolicy::Wrap,
    };
    let (_, begin_ref) = attached.push(AttachedFunction::BEGIN_LABEL, new_op_2.clone().into());

    assert_eq!(begin_ref.block, AttachedFunction::BEGIN_LABEL);
    assert_eq!(attached.get(begin_ref).cloned(), Some(new_op_2.into()));

    let end_dest = attached.next_available_name();
    let new_op_3 = IAdd {
        dest: end_dest,
        ty: i32_ty,
        lhs: op_a,
        rhs: op_b,
        variant: OverflowSignednessPolicy::UTrap,
    };
    let (_, end_ref) = attached.push(AttachedFunction::END_LABEL, new_op_3.clone().into());
    assert_eq!(end_ref.block, AttachedFunction::END_LABEL);
    assert_eq!(attached.get(end_ref).cloned(), Some(new_op_3.into()));
}

#[test]
fn attached_function_pop_respects_dependency_counters() {
    let (function, type_registry) = build_chain_function();
    let mut attached = AttachedFunction::new(function.clone());
    let entry = function
        .body
        .get(&Label::NIL)
        .expect("entry block should exist");

    let op_twice = entry.instructions[0]
        .destination()
        .expect("first instruction should define a value");
    let op_plus_one = entry.instructions[1]
        .destination()
        .expect("second instruction should define a value");
    let i32_ty = type_registry.search_or_insert(IType::I32.into());

    let first_dest = attached.next_available_name();
    let first_overlay: HyInstr = IMul {
        dest: first_dest,
        ty: i32_ty,
        lhs: Operand::Reg(op_twice),
        rhs: Operand::Reg(op_plus_one),
        variant: OverflowSignednessPolicy::Wrap,
    }
    .into();
    let expected_first = first_overlay.clone();
    let (_, first_ref) = attached.push(Label::NIL, first_overlay);

    let second_dest = attached.next_available_name();
    let second_overlay: HyInstr = IAdd {
        dest: second_dest,
        ty: i32_ty,
        lhs: Operand::Reg(first_dest),
        rhs: Operand::Reg(op_plus_one),
        variant: OverflowSignednessPolicy::Wrap,
    }
    .into();
    let expected_second = second_overlay.clone();
    let (_, second_ref) = attached.push(Label::NIL, second_overlay);

    let pop_dependency_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let _ = attached.pop(first_ref);
    }));
    assert!(
        pop_dependency_result.is_err(),
        "popping an instruction with dependents should panic"
    );

    let popped_second = attached.pop(second_ref);
    assert_eq!(popped_second, expected_second);

    let popped_first = attached.pop(first_ref);
    assert_eq!(popped_first, expected_first);
}

#[test]
fn attached_function_dedup_correctly() {
    let (function, type_registry) = build_chain_function();
    let op_a = function
        .body
        .get(&Label::NIL)
        .expect("entry block should exist")
        .instructions[0]
        .destination()
        .unwrap();
    let op_b = function
        .body
        .get(&Label::NIL)
        .expect("entry block should exist")
        .instructions[1]
        .destination()
        .unwrap();

    let i32_ty = type_registry.search_or_insert(IType::I32.into());

    // Perform the operation imul %a, %b (should be unique, therefore not deduped)
    let mut attached = AttachedFunction::new(Arc::clone(&function));
    let dest_a = attached.next_available_name();
    let instr_unique = IMul {
        dest: dest_a,
        ty: i32_ty,
        lhs: Operand::Reg(op_a),
        rhs: Operand::Reg(op_b),
        variant: OverflowSignednessPolicy::Wrap,
    };
    let (dest, instr_unique_ref) = attached.push(Label::NIL, instr_unique.into());
    assert_eq!(dest, Some(dest_a));
    assert_ne!(instr_unique_ref.reserved, 0);

    // Perform the operation imul %a, %b again (but with different variant, should NOT be deduped)
    let dest_b = attached.next_available_name();
    let instr_not_deduped = IMul {
        dest: dest_b,
        ty: i32_ty,
        lhs: Operand::Reg(op_a),
        rhs: Operand::Reg(op_b),
        variant: OverflowSignednessPolicy::USat,
    };
    let (dest, instr_not_deduped_ref) = attached.push(Label::NIL, instr_not_deduped.into());
    assert_eq!(dest, Some(dest_b));
    assert_ne!(instr_not_deduped_ref.reserved, 0);
    assert_ne!(instr_not_deduped_ref, instr_unique_ref);

    // Perform the operation imul %a, %b again (should be deduped)
    let dest_c = attached.next_available_name();
    let instr_deduped = IMul {
        dest: dest_c,
        ty: i32_ty,
        lhs: Operand::Reg(op_a),
        rhs: Operand::Reg(op_b),
        variant: OverflowSignednessPolicy::Wrap,
    };
    let (dest, instr_deduped_ref) = attached.push(Label::NIL, instr_deduped.into());
    assert_eq!(dest, Some(dest_a));
    assert_ne!(dest, Some(dest_c));
    assert_eq!(instr_deduped_ref, instr_unique_ref,);
}

#[test]
fn attached_function_do_not_dedup_non_simple() {
    let (function, type_registry) = build_chain_function();
    let op_a = function
        .body
        .get(&Label::NIL)
        .expect("entry block should exist")
        .instructions[0]
        .destination()
        .unwrap();
    let i32_ty = type_registry.search_or_insert(IType::I32.into());
    let ptr_ty = type_registry.search_or_insert(hyinstr::types::primary::PtrType.into());

    // Cast op_a to ptr using

    // Perform the operation idiv %a, %b (should be unique, therefore not deduped)
    let mut attached = AttachedFunction::new(Arc::clone(&function));
    let op_b = {
        let dest = attached.next_available_name();
        let instr = Cast {
            dest,
            ty: ptr_ty,
            value: Operand::Reg(op_a),
            variant: hyinstr::modules::instructions::misc::CastVariant::IntToPtr,
        };
        let (new_dest, cast_ref) = attached.push(Label::NIL, instr.into());
        let new_dest = new_dest.expect("cast instruction should define a value");
        assert_eq!(new_dest, dest);
        assert_ne!(cast_ref.reserved, 0);
        dest
    };

    // Perform the operation mload %b (should NOT be deduped)
    let dest_a = attached.next_available_name();
    let instr_non_simple = MLoad {
        dest: dest_a,
        ty: i32_ty,
        addr: Operand::Reg(op_b),
        alignement: Some(16),
        ordering: None,
        volatile: false,
    };
    let (dest, instr_non_simple_ref) = attached.push(Label::NIL, instr_non_simple.into());
    assert_ne!(instr_non_simple_ref.reserved, 0);
    assert_eq!(dest, Some(dest_a));

    // Perform the exact same mload %b again (should NOT be deduped because memory ops are non-simple)
    let dest_b = attached.next_available_name();
    let instr_not_deduped = MLoad {
        dest: dest_b,
        ty: i32_ty,
        addr: Operand::Reg(op_b),
        alignement: Some(16),
        ordering: None,
        volatile: false,
    };
    let (dest, instr_not_deduped_ref) = attached.push(Label::NIL, instr_not_deduped.into());
    assert_eq!(dest, Some(dest_b));
    assert_ne!(instr_not_deduped_ref.reserved, 0);
    assert_ne!(instr_not_deduped_ref, instr_non_simple_ref);
}
