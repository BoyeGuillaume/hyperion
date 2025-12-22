use hycore::specifications::utils::{remove_unused_op, simple_simplify_function};
use hyinstr::{
    modules::{Module, parser::extend_module_from_string},
    types::TypeRegistry,
};

const FN_CODE: &str = r#"
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

fn main() {
    let registry = TypeRegistry::new([0; 6]);
    let mut module = Module::default();

    if let Err(e) = extend_module_from_string(&mut module, &registry, FN_CODE) {
        eprintln!("Error parsing module: {}", e);
        return;
    }

    let func_uuid = module
        .find_internal_function_uuid_by_name("factorial")
        .unwrap();
    let func = module.get_internal_function_by_uuid_mut(func_uuid).unwrap();

    // let mut func = parse_result.into_output().unwrap();
    simple_simplify_function(func).unwrap();
    remove_unused_op(func).unwrap();

    println!("Parsed module:");
    println!("====================");
    println!("{}", module.fmt(&registry));
    println!("====================");
}
