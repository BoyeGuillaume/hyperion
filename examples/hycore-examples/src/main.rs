use hycore::specifications::utils::{remove_unused_op, simple_simplify_function};
use hyinstr::{
    modules::{Function, Module, parser::extend_module_from_string, symbol::FunctionPointerType},
    types::TypeRegistry,
};
use uuid::Uuid;

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

    println!("Parsed module: {}", module.fmt(&registry));
}
