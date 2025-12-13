use chumsky::Parser;
use hycore::specifications::utils::{remove_unused_op, simple_simplify_function};
use hyinstr::{
    modules::{Module, parser::function_parser, symbol::FunctionPointerType},
    types::TypeRegistry,
};
use uuid::Uuid;

const FN_CODE: &str = r#"
define i32 %factorial ( %n: i32 ) {
entry:
   %cmp1 = icmp eq i1 %n, i32 0
   branch %cmp1, return_result, recurse
recurse:
   %n_minus_1 = isub wrap unsigned i32 %n, i32 1
   %recursive_result = invoke i32 ptr %factorial (%n_minus_1)
   %result2 = imul saturate unsigned i32 %n, %recursive_result
   %result = imul wrap unsigned i32 %n, %recursive_result
   jump return_result
return_result:
   %final_result = phi i32 [ recurse, %result2 ], [ entry, i32 1 ]
   ret %final_result
}
"#;

fn main() {
    let type_registry = TypeRegistry::new([0; 6]);
    let uuid = Uuid::new_v4();
    let func_retriever = |name: String, func_type: FunctionPointerType| -> Option<Uuid> {
        if name == "factorial" && func_type == FunctionPointerType::Internal {
            Some(uuid)
        } else {
            None
        }
    };

    let parse_result = function_parser(func_retriever, &type_registry, uuid).parse(FN_CODE);
    if parse_result.has_errors() {
        for err in parse_result.errors() {
            let (start, end) = {
                let s = err.span();
                (s.start, s.end)
            };

            eprintln!("Parse error: {}", err);
            /* Display line with error */
            let line_start = FN_CODE[..start].rfind('\n').map_or(0, |p| p + 1);
            let line_end = FN_CODE[end..].find('\n').map_or(FN_CODE.len(), |p| end + p);
            eprintln!("{}", &FN_CODE[line_start..line_end]);
            eprintln!("{:>width$}^", "", width = start - line_start);
        }
        panic!("Failed to parse function code");
    }

    let mut func = parse_result.into_output().unwrap();
    func.verify().expect("Function verification failed");

    simple_simplify_function(&mut func).unwrap();
    remove_unused_op(&mut func).unwrap();

    let mut module = Module::default();
    module.functions.insert(func.uuid, func);
    // module.verify().unwrap();

    println!("Parsed module: {}", module.fmt(&type_registry));
}
