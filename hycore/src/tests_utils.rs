use hyinstr::{
    modules::{Module, parser::extend_module_from_string},
    types::TypeRegistry,
};

/// Example function for testing purposes (function pow(x, n) = x^n)
pub fn example_a(registry: &mut TypeRegistry, module: &mut Module) {
    extend_module_from_string(
        module,
        registry,
        r#"
    
    "#,
    )
    .unwrap();
    todo!()
    // extend_module_from_string(
    //     module,
    //     registry,
    //     r#"
    // define i32 %pow(%x: i32) {
    // entry:
    //     %is_zero = icmp eq i1 %n, i32 0
    //     branch %is_zero, return, return
    // return:
    //     %ret_val = phi i32 [i32 1, entry]
    //     ret %ret_val
    // }
    // "#,
    // )
    // .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_a() {
        let mut registry = TypeRegistry::new([0; 6]);
        let mut module = Module::default();
        example_a(&mut registry, &mut module);
        module.verify().unwrap();
    }
}
