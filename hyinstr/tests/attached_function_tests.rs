use std::{panic, sync::Arc};

use hyinstr::{
    attached::AttachedFunction,
    modules::{
        Function, Module,
        instructions::Instruction,
        operand::{Label, Name, Operand},
        parser::extend_module_from_string,
    },
    types::TypeRegistry,
};

const CHAIN_SOURCE: &str = r#"
define i32 overlay_inputs(%x: i32) {
entry:
    %twice: i32 = iadd.wrap %x, %x
    %plus_one: i32 = iadd.wrap %twice, i32 1
    ret %plus_one
}
"#;

fn build_chain_function() -> Arc<Function> {
    let mut module = Module::default();
    let registry = TypeRegistry::new([0; 6]);
    extend_module_from_string(&mut module, &registry, CHAIN_SOURCE)
        .expect("failed to parse sample function");

    module
        .functions
        .values()
        .next()
        .cloned()
        .expect("sample function present")
}

#[test]
fn attached_function_initializes_counters_from_target() {
    let function = build_chain_function();
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
    let function = build_chain_function();
    let mut attached = AttachedFunction::new(Arc::clone(&function));
    let entry = function
        .body
        .get(&Label::NIL)
        .expect("entry block should exist");

    let mut overlay_instr = entry.instructions[0].clone();
    let overlay_dest = attached.next_available_name();
    overlay_instr.set_destination(overlay_dest);
    let overlay_expected = overlay_instr.clone();
    let overlay_ref = attached.push(Label::NIL, overlay_instr);

    assert_ne!(overlay_ref.reserved, 0);
    assert_eq!(attached.find_by_dest(&overlay_dest), Some(overlay_ref));
    assert_eq!(attached.get(overlay_ref).cloned(), Some(overlay_expected));

    let mut begin_instr = entry.instructions[1].clone();
    let begin_dest = attached.next_available_name();
    begin_instr.set_destination(begin_dest);
    let begin_expected = begin_instr.clone();
    let begin_ref = attached.push(AttachedFunction::BEGIN_LABEL, begin_instr);

    assert_eq!(begin_ref.block, AttachedFunction::BEGIN_LABEL);
    assert_eq!(attached.get(begin_ref).cloned(), Some(begin_expected));

    let mut end_instr = entry.instructions[0].clone();
    let end_dest = attached.next_available_name();
    end_instr.set_destination(end_dest);
    let end_expected = end_instr.clone();
    let end_ref = attached.push(AttachedFunction::END_LABEL, end_instr);

    assert_eq!(end_ref.block, AttachedFunction::END_LABEL);
    assert_eq!(attached.get(end_ref).cloned(), Some(end_expected));
}

#[test]
fn attached_function_pop_respects_dependency_counters() {
    let function = build_chain_function();
    let mut attached = AttachedFunction::new(function.clone());
    let entry = function
        .body
        .get(&Label::NIL)
        .expect("entry block should exist");
    let original_first_dest = entry.instructions[0]
        .destination()
        .expect("first instruction should define a value");

    let mut first_overlay = entry.instructions[0].clone();
    let first_dest = attached.next_available_name();
    first_overlay.set_destination(first_dest);
    let expected_first = first_overlay.clone();
    let first_ref = attached.push(Label::NIL, first_overlay);

    let mut second_overlay = entry.instructions[1].clone();
    for operand in second_overlay.operands_mut() {
        if let Operand::Reg(reg) = operand {
            if *reg == original_first_dest {
                *reg = first_dest;
            }
        }
    }
    let second_dest = attached.next_available_name();
    second_overlay.set_destination(second_dest);
    let expected_second = second_overlay.clone();
    let second_ref = attached.push(Label::NIL, second_overlay);

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
