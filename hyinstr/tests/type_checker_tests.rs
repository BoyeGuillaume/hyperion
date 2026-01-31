use hyinstr::{
    modules::parser::extend_module_from_string,
    modules::{Function, Module},
    types::TypeRegistry,
    utils::Error,
};

fn registry() -> TypeRegistry {
    TypeRegistry::new([0; 6])
}

fn parse_module(registry: &TypeRegistry, source: &str) -> Module {
    let mut module = Module::default();
    extend_module_from_string(&mut module, registry, source).expect("failed to parse IR");
    module
}

fn get_function<'a>(module: &'a Module, name: &str) -> &'a Function {
    let uuid = module
        .find_internal_function_uuid_by_name(name)
        .unwrap_or_else(|| panic!("function `{name}` not found"));
    module
        .get_internal_function_by_uuid(uuid)
        .unwrap_or_else(|| panic!("function `{name}` missing body"))
}

fn expect_type_mismatch(result: Result<(), Error>) {
    let err = result.unwrap_err();
    assert!(
        matches!(err, Error::TypeMismatch { .. }),
        "expected type mismatch, got {err:?}"
    );
}

fn expect_elem_index_oob(result: Result<(), Error>) {
    let err = result.unwrap_err();
    assert!(
        matches!(err, Error::ElementIndexOutOfBounds { .. }),
        "expected element index out of bounds, got {err:?}"
    );
}

#[test]
fn iadd_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 add(%x: i32, %y: i32) {
entry:
    %sum: i32 = iadd.wrap %x, %y
    ret %sum
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "add")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 add_bad(%x: fp32) {
entry:
    %sum: i33 = iadd.wrap %x, fp32 1.0
    ret %sum
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "add_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn isub_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 sub(%x: i32, %y: i32) {
entry:
    %diff: i32 = isub.wrap %x, %y
    ret %diff
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "sub")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 sub_bad(%x: fp32, %y: fp32) {
entry:
    %diff: fp32 = isub.wrap %x, %y
    ret %diff
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "sub_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn imul_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define <4 x i32> mul(%x: <4 x i32>, %y: <4 x i32>) {
entry:
    %prod: <4 x i32> = imul.wrap %x, %y
    ret %prod
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "mul")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define <4 x i32> mul_bad(%x: <4 x fp32>, %y: <4 x fp32>) {
entry:
    %prod: <4 x fp32> = imul.wrap %x, %y
    ret %prod
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "mul_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn idiv_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 div(%x: i32, %y: i32) {
entry:
    %quot: i32 = idiv.signed %x, %y
    ret %quot
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "div")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define <4 x i32> div_bad(%x: <4 x fp32>, %y: <4 x fp32>) {
entry:
    %quot: <3 x i32> = idiv.signed %x, %y
    ret %quot
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "div_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn irem_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 rem(%x: i32, %y: i32) {
entry:
    %rem: i32 = irem.signed %x, %y
    ret %rem
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "rem")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 rem_bad(%x: fp32, %y: fp32) {
entry:
    %rem: fp32 = irem.signed %x, %y
    ret %rem
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "rem_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn icmp_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i1 cmp(%x: i32, %y: i32) {
entry:
    %eq: i1 = icmp.eq %x, %y
    ret %eq
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "cmp")
            .type_check(&registry)
            .is_ok()
    );

    let ok_ir2 = r#"
define <4 x i1> cmp_vec(%x: <4 x i32>, %y: <4 x i32>) {
entry:  
    %eq: <4 x i1> = icmp.eq %x, %y
    ret %eq
}
"#;
    let ok_module2 = parse_module(&registry, ok_ir2);
    assert!(
        get_function(&ok_module2, "cmp_vec")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 cmp_bad(%x: i32, %y: i32) {
entry:
    %eq: i32 = icmp.eq %x, %y
    ret %eq
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "cmp_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn isht_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 shl(%value: i32, %amount: i32) {
entry:
    %out: i32 = isht.lsl %value, %amount
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "shl")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 shl_bad(%value: i32) {
entry:
    %out: i32 = isht.lsl %value, fp32 1.0
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "shl_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn ineg_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 negate(%value: i32) {
entry:
    %out: i32 = ineg %value
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "negate")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 negate_bad(%value: fp32) {
entry:
    %out: fp32 = ineg %value
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "negate_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn and_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 bit_and(%lhs: i32, %rhs: i32) {
entry:
    %out: i32 = and %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "bit_and")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 bit_and_bad(%lhs: i32) {
entry:
    %out: i32 = and %lhs, fp32 0.0
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "bit_and_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn or_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 bit_or(%lhs: i32, %rhs: i32) {
entry:
    %out: i32 = or %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "bit_or")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 bit_or_bad(%lhs: i32) {
entry:
    %out: i32 = or %lhs, fp32 0.0
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "bit_or_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn xor_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 bit_xor(%lhs: i32, %rhs: i32) {
entry:
    %out: i32 = xor %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "bit_xor")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 bit_xor_bad(%lhs: i32) {
entry:
    %out: i32 = xor %lhs, fp32 0.0
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "bit_xor_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn not_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 invert(%value: i32) {
entry:
    %out: i32 = not %value
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "invert")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 invert_bad(%value: fp32) {
entry:
    %out: fp32 = not %value
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "invert_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn implies_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i1 implies_ok(%cond: i1, %then: i1) {
entry:
    %out: i1 = implies %cond, %then
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "implies_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i1 implies_bad(%cond: i1) {
entry:
    %out: i1 = implies %cond, i32 1
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "implies_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn equiv_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i1 equiv_ok(%lhs: i1, %rhs: i1) {
entry:
    %out: i1 = equiv %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "equiv_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i1 equiv_bad(%lhs: i1) {
entry:
    %out: i1 = equiv %lhs, fp32 0.0
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "equiv_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn fadd_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define fp32 fadd_ok(%lhs: fp32, %rhs: fp32) {
entry:
    %out: fp32 = fadd %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "fadd_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 fadd_bad(%lhs: fp32, %rhs: fp32) {
entry:
    %out: i32 = fadd %lhs, %rhs
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "fadd_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn fsub_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define fp32 fsub_ok(%lhs: fp32, %rhs: fp32) {
entry:
    %out: fp32 = fsub %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "fsub_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define fp32 fsub_bad(%lhs: i32, %rhs: fp32) {
entry:
    %out: fp32 = fsub %lhs, %rhs
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "fsub_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn fmul_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define fp32 fmul_ok(%lhs: fp32, %rhs: fp32) {
entry:
    %out: fp32 = fmul %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "fmul_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 fmul_bad(%lhs: fp32, %rhs: fp32) {
entry:
    %out: i32 = fmul %lhs, %rhs
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "fmul_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn fdiv_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define fp32 fdiv_ok(%lhs: fp32, %rhs: fp32) {
entry:
    %out: fp32 = fdiv %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "fdiv_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 fdiv_bad(%lhs: fp32, %rhs: fp32) {
entry:
    %out: i32 = fdiv %lhs, %rhs
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "fdiv_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn frem_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define fp32 frem_ok(%lhs: fp32, %rhs: fp32) {
entry:
    %out: fp32 = frem %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "frem_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 frem_bad(%lhs: fp32, %rhs: fp32) {
entry:
    %out: i32 = frem %lhs, %rhs
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "frem_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn fcmp_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i1 fcmp_ok(%lhs: fp32, %rhs: fp32) {
entry:
    %out: i1 = fcmp.oeq %lhs, %rhs
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "fcmp_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 fcmp_bad(%lhs: fp32, %rhs: fp32) {
entry:
    %out: i32 = fcmp.oeq %lhs, %rhs
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "fcmp_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn fneg_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define fp32 fneg_ok(%value: fp32) {
entry:
    %out: fp32 = fneg %value
    ret %out
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "fneg_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 fneg_bad(%value: fp32) {
entry:
    %out: i32 = fneg %value
    ret %out
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "fneg_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn mload_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 load_ok(%src: ptr) {
entry:
    %val: i32 = load.acq_rel %src, volatile
    ret %val
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "load_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 load_bad(%src: i32) {
entry:
    %val: i32 = load %src
    ret %val
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "load_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn mstore_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define void store_ok(%dst: ptr, %value: i32) {
entry:
    store %dst, %value
    store.release %dst, %value, align 0x10, volatile
    ret void
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "store_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define void store_bad(%dst: i32, %value: i32) {
entry:
    store %dst, %value
    ret void
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "store_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn malloca_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define ptr alloca_ok(%count: i32) {
entry:
    %buf: ptr = alloca %count
    ret %buf
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "alloca_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define ptr alloca_bad() {
entry:
    %buf: ptr = alloca fp32 1.0
    ret %buf
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "alloca_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn mgetelementptr_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define ptr gep_ok(%base: ptr, %idx: i32) {
entry:
    %offset: ptr = getelementptr i32, %base, %idx
    ret %offset
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "gep_ok")
            .type_check(&registry)
            .is_ok()
    );

    let ok_ir2 = r#"
define fp32 gep_ok2(%base: ptr, %idx: i32) {
entry:
    %offset: ptr = getelementptr { ptr, fp32 }, %base, %idx, i32 1
    %value: fp32 = load %offset
    ret %value
}"#;
    let ok_module2 = parse_module(&registry, ok_ir2);
    assert!(
        get_function(&ok_module2, "gep_ok2")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define ptr gep_bad(%base: ptr, %idx: fp32) {
entry:
    %offset: ptr = getelementptr i32, %base, %idx
    ret %offset
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "gep_bad");
    expect_type_mismatch(bad_func.type_check(&registry));

    let bad_ir2 = r#"
define ptr gep_bad2(%base: i32, %idx: i32) {
entry:
    %offset: ptr = getelementptr { ptr, i32 }, %base, %idx
    ret %offset
}"#;
    let bad_module2 = parse_module(&registry, bad_ir2);
    let bad_func2 = get_function(&bad_module2, "gep_bad2");
    expect_type_mismatch(bad_func2.type_check(&registry));
}

#[test]
fn invoke_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 callee(%x: i32) {
entry:
    ret %x
}

define i32 caller(%value: i32) {
entry:
    %result: i32 = invoke ptr callee, %value
    ret %result
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "caller")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 invoke_bad(%fn: i32) {
entry:
    %result: i32 = invoke %fn, %fn
    ret %result
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "invoke_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn phi_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 phi_ok(%x: i32, %y: i32, %cond: i1) {
entry:
    branch %cond, left, right

left:
    jump join

right:
    jump join

join:
    %value: i32 = phi [ %x, left ], [ %y, right ]
    ret %value
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "phi_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 phi_bad(%x: i32, %cond: i1) {
entry:
    branch %cond, left, right

left:
    jump join

right:
    jump join

join:
    %value: i32 = phi [ %x, left ], [ fp32 1.0, right ]
    ret %value
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "phi_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn select_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 select_ok(%cond: i1, %lhs: i32, %rhs: i32) {
entry:
    %value: i32 = select %cond, %lhs, %rhs
    ret %value
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "select_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 select_bad(%cond: i32, %lhs: i32, %rhs: i32) {
entry:
    %value: i32 = select %cond, %lhs, %rhs
    ret %value
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "select_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn cast_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 cast_ok(%value: i16) {
entry:
    %wide: i32 = cast.zext %value
    ret %wide
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "cast_ok")
            .type_check(&registry)
            .is_ok()
    );

    let ok_ir2 = r#"
define i16 cast_ok2(%value: i32) {
entry:
    %narrow: i16 = cast.trunc %value
    ret %narrow
}
"#;
    let ok_module2 = parse_module(&registry, ok_ir2);
    assert!(
        get_function(&ok_module2, "cast_ok2")
            .type_check(&registry)
            .is_ok()
    );

    let ok_ir3 = r#"
define fp32 cast_ok3(%value: i32) {
entry:
    %fvalue: fp32 = cast.sitofp %value
    ret %fvalue
}
"#;
    let ok_module3 = parse_module(&registry, ok_ir3);
    assert!(
        get_function(&ok_module3, "cast_ok3")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 cast_bad(%value: i64) {
entry:
    %wide: i32 = cast.zext %value
    ret %wide
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "cast_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn insertvalue_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define { ptr, i32 } insert_ok(%i_ptr: ptr, %i_len: i32) {
entry:
    %updated: { ptr, i32 } = insertvalue { ptr, i32 } undef, %i_ptr, i32 0
    %updated2: { ptr, i32 } = insertvalue %updated, %i_len, i32 1
    ret %updated2
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "insert_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define { ptr, i32 } insert_bad(%pair: { ptr, i32 }) {
entry:
    %updated: { ptr, i32 } = insertvalue %pair, i32 1, i32 0
    ret %updated
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "insert_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn extractvalue_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define ptr extract_ok(%pair: { ptr, i32 }) {
entry:
    %ptr: ptr = extractvalue %pair, i32 0
    %i: i32 = extractvalue %pair, i32 1
    ret %ptr
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "extract_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 extract_bad(%pair: { ptr, i32 }) {
entry:
    %ptr: i32 = extractvalue %pair, i32 0
    ret %ptr
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "extract_bad");
    expect_type_mismatch(bad_func.type_check(&registry));

    let bad_ir2 = r#"
define i32 extract_bad2(%pair: { ptr, i32 }) {
entry:
    %i: i32 = extractvalue %pair, i32 2
    ret %i
}
"#;
    let bad_module2 = parse_module(&registry, bad_ir2);
    let bad_func2 = get_function(&bad_module2, "extract_bad2");
    expect_elem_index_oob(bad_func2.type_check(&registry));
}

#[test]
fn meta_assert_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define void !assert_ok(%cond: i1) {
entry:
    !assert %cond
    ret void
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "assert_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define void !assert_bad(%cond: i32) {
entry:
    !assert %cond
    ret void
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "assert_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn meta_assume_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define void !assume_ok(%cond: i1) {
entry:
    !assume %cond
    ret void
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "assume_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define void !assume_bad(%cond: i32) {
entry:
    !assume %cond
    ret void
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "assume_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn meta_isdef_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i1 !isdef_ok(%value: i32) {
entry:
    %flag: i1 = !isdef %value
    ret %flag
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "isdef_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 !isdef_bad(%value: i32) {
entry:
    %flag: i32 = !isdef %value
    ret %flag
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "isdef_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn meta_prob_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define fp32 !prob_ok(%value: i32) {
entry:
    %expected: fp32 = !prob.xpt %value
    ret %expected
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "prob_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i32 !prob_bad(%value: i32) {
entry:
    %expected: i32 = !prob.xpt %value
    ret %expected
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "prob_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn meta_analysis_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define i32 !analysis_ok() {
entry:
    %count: i32 = !analysis.icnt i32 0x1
    ret %count
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "analysis_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define i1 !analysis_bad() {
entry:
    %count: i1 = !analysis.icnt i32 0x1
    ret %count
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "analysis_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn meta_forall_test_type_checks() {
    let registry = registry();

    let ok_ir = r#"
define void !forall_ok() {
entry:
    %claim: i1 = !forall
    !assume %claim
    ret void
}
"#;
    let ok_module = parse_module(&registry, ok_ir);
    assert!(
        get_function(&ok_module, "forall_ok")
            .type_check(&registry)
            .is_ok()
    );

    let bad_ir = r#"
define void !forall_bad() {
entry:
    %claim: i32 = !forall
    !assume %claim
    ret void
}
"#;
    let bad_module = parse_module(&registry, bad_ir);
    let bad_func = get_function(&bad_module, "forall_bad");
    expect_type_mismatch(bad_func.type_check(&registry));
}

#[test]
fn factorial_module_type_checks() {
    let registry = registry();

    let factorial_ir = r#"
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

define void !factorial_test_a (%n: i32) {
entry:
    %n_less_1: i32 = isub.wrap %n, i32 1
    %n_greater_0: i1 = icmp.ugt %n, i32 0
    !assume %n_greater_0
    %fact_n: i32 = invoke ptr factorial, %n
    %fact_n_minus_0: i32 = invoke ptr factorial, %n_less_1

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

    let module = parse_module(&registry, factorial_ir);
    module
        .type_check(&registry)
        .expect("factorial IR should type check");
}

#[test]
fn pipeline_module_rejects_void_return_in_i32_function() {
    let registry = registry();

    let ir = r#"
define i32 pipeline(%n: i32, %m: i32, %limit: i32) {
entry:
    %sum: i32 = iadd.wrap %n, %m
    %gt: i1 = icmp.ugt %sum, %limit
    branch %gt, saturate, accumulate

saturate:
    %clamped: i32 = select %gt, %limit, %sum
    jump exit

accumulate:
    %diff: i32 = isub.wrap %limit, %sum
    %mul: i32 = imul.wrap %diff, %m
    jump exit

exit:
    %result: i32 = phi [ %clamped, saturate ], [ %mul, accumulate ]
    ret void
}
"#;

    let module = parse_module(&registry, ir);
    let err = module.type_check(&registry).unwrap_err();
    assert!(matches!(err, Error::TypeMismatch { .. }));
}
