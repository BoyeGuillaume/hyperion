use hyinstr::{
    modules::{Module, instructions::HyInstr, parser::extend_module_from_string},
    types::TypeRegistry,
    utils::Error,
};

fn registry() -> TypeRegistry {
    TypeRegistry::new([0; 6])
}

#[test]
fn parser_handles_richer_math_examples() {
    let reg = registry();
    let mut module = Module::default();

    let source = r#"
define fp32 dot3(%a: <3 x fp32>, %b: <3 x fp32>) {
entry:
    %ax: fp32 = extractvalue %a, i32 0
    %ay: fp32 = extractvalue %a, i32 1
    %az: fp32 = extractvalue %a, i32 2
    %bx: fp32 = extractvalue %b, i32 0
    %by: fp32 = extractvalue %b, i32 1
    %bz: fp32 = extractvalue %b, i32 2
    %m0: fp32 = fmul %ax, %bx
    %m1: fp32 = fmul %ay, %by
    %m2: fp32 = fmul %az, %bz
    %s0: fp32 = fadd %m0, %m1
    %s1: fp32 = fadd %s0, %m2
    ret %s1
}

define fp32 pow_fp32(%x: fp32, %exp: i32) {
entry:
    %exp_is_zero: i1 = icmp.eq %exp, i32 0
    branch %exp_is_zero, pow_zero, pow_loop

pow_zero:
    ret fp32 1.0

pow_loop:
    %acc: fp32 = phi [ fp32 1.0, entry ], [ %acc_next, pow_iter ]
    %e: i32 = phi [ %exp, entry ], [ %e_next, pow_iter ]
    %done: i1 = icmp.eq %e, i32 0
    branch %done, pow_exit, pow_iter

pow_iter:
    %acc_next: fp32 = fmul %acc, %x
    %e_next: i32 = isub.wrap %e, i32 1
    jump pow_loop

pow_exit:
    ret %acc
}

define fp32 sqrt_newton(%x: fp32) {
entry:
    %half_x: fp32 = fmul %x, fp32 0.5
    %guess0: fp32 = fadd %half_x, fp32 1.0
    %reciprocal: fp32 = fdiv %x, %guess0
    %avg: fp32 = fadd %guess0, %reciprocal
    %guess1: fp32 = fmul %avg, fp32 0.5
    %reciprocal2: fp32 = fdiv %x, %guess1
    %avg2: fp32 = fadd %guess1, %reciprocal2
    %guess2: fp32 = fmul %avg2, fp32 0.5
    ret %guess2
}

define fp32 dot_dynamic(%a: { ptr, i32 }, %b: { ptr, i32 }) {
entry:
    %a_data: ptr = extractvalue %a, i32 0
    %a_len: i32 = extractvalue %a, i32 1
    %b_data: ptr = extractvalue %b, i32 0
    %b_len: i32 = extractvalue %b, i32 1
    %same_len: i1 = icmp.eq %a_len, %b_len
    branch %same_len, dot_loop, dot_empty

dot_empty:
    ret fp32 0.0

dot_loop:
    %acc: fp32 = phi [ fp32 0.0, entry ], [ %acc_next, dot_iter ]
    %idx: i32 = phi [ i32 0, entry ], [ %idx_next, dot_iter ]
    %done: i1 = icmp.eq %idx, %a_len
    branch %done, dot_exit, dot_iter

dot_iter:
    %offset_a: ptr = getelementptr fp32, %a_data, %idx
    %offset_b: ptr = getelementptr fp32, %b_data, %idx
    %aval: fp32 = load %offset_a, align 4
    %bval: fp32 = load %offset_b, align 4
    %prod: fp32 = fmul %aval, %bval
    %acc_next: fp32 = fadd %acc, %prod
    %idx_next: i32 = iadd.wrap %idx, i32 1
    jump dot_loop

dot_exit:
    ret %acc
}

define i32 max_u32(%data: ptr, %len: i32) {
entry:
    %empty: i1 = icmp.eq %len, i32 0
    branch %empty, max_zero, max_init

max_zero:
    ret i32 0

max_init:
    %first_ptr: ptr = getelementptr i32, %data, i32 0
    %current: i32 = load %first_ptr
    jump max_loop

max_loop:
    %max: i32 = phi [ %current, max_init ], [ %max_next, max_iter ]
    %idx: i32 = phi [ i32 1, max_init ], [ %idx_next, max_iter ]
    %done: i1 = icmp.eq %idx, %len
    branch %done, max_exit, max_iter

max_iter:
    %ptr_i: ptr = getelementptr i32, %data, %idx
    %value: i32 = load %ptr_i
    %cmp: i1 = icmp.ugt %value, %max
    %max_next: i32 = select %cmp, %value, %max
    %idx_next: i32 = iadd.wrap %idx, i32 1
    jump max_loop

max_exit:
    ret %max
}

define i32 clamp_i32(%x: i32, %lo: i32, %hi: i32) {
entry:
    %lt_lo: i1 = icmp.slt %x, %lo
    %gt_hi: i1 = icmp.sgt %x, %hi
    %lo_or_x: i32 = select %lt_lo, %lo, %x
    %clamped: i32 = select %gt_hi, %hi, %lo_or_x
    ret %clamped
}
"#;

    if let Some(e) = extend_module_from_string(&mut module, &reg, source).err() {
        match e {
            Error::ParserErrors { errors, tokens } => {
                for err in errors {
                    // Get line before and after the error for context
                    let mut context_start = err.start;
                    let mut context_end = err.end;
                    while context_start > 0 && &source[context_start - 1..context_start] != "\n" {
                        context_start -= 1;
                    }
                    while context_end < source.len()
                        && &source[context_end..context_end + 1] != "\n"
                    {
                        context_end += 1;
                    }
                    let context = &source[context_start..context_end];
                    eprintln!("Context:\n{}", context);

                    eprintln!("Parser error: {:?}", err);
                }
                eprintln!("Tokens: {:?}", tokens);
                panic!("Failed to parse math examples due to parser errors.");
            }
            _ => {
                panic!("Failed to parse math examples: {}", e);
            }
        }
    }

    for name in [
        "dot3",
        "pow_fp32",
        "sqrt_newton",
        "dot_dynamic",
        "max_u32",
        "clamp_i32",
    ] {
        assert!(module.find_internal_function_uuid_by_name(name).is_some());
    }

    module.verify().unwrap();

    let dot_uuid = module
        .find_internal_function_uuid_by_name("dot_dynamic")
        .unwrap();
    let dot_func = module.get_internal_function_by_uuid(dot_uuid).unwrap();
    let phi_count = dot_func
        .iter()
        .filter(|(instr, _)| matches!(instr, HyInstr::Phi(_)))
        .count();
    assert!(phi_count >= 2);
    assert!(
        dot_func
            .iter()
            .any(|(instr, _)| matches!(instr, HyInstr::MGetElementPtr(_)))
    );

    let max_uuid = module
        .find_internal_function_uuid_by_name("max_u32")
        .unwrap();
    let max_func = module.get_internal_function_by_uuid(max_uuid).unwrap();
    assert!(
        max_func
            .iter()
            .any(|(instr, _)| matches!(instr, HyInstr::Select(_)))
    );

    // Basic sanity: pow depends on fmul/isub and clamp uses comparisons.
    let pow_uuid = module
        .find_internal_function_uuid_by_name("pow_fp32")
        .unwrap();
    let pow_func = module.get_internal_function_by_uuid(pow_uuid).unwrap();
    assert!(
        pow_func
            .iter()
            .any(|(instr, _)| matches!(instr, HyInstr::FDiv(_) | HyInstr::FMul(_)))
    );

    let clamp_uuid = module
        .find_internal_function_uuid_by_name("clamp_i32")
        .unwrap();
    let clamp_func = module.get_internal_function_by_uuid(clamp_uuid).unwrap();
    assert!(
        clamp_func
            .iter()
            .any(|(instr, _)| matches!(instr, HyInstr::Select(_)))
    );
}
