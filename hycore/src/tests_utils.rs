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
    define i32 foo(%a: i32, %b: i32) {
    entry:
        %result: i32 = iadd.usat %a, %b
        ret %result
    }
    "#,
    )
    .unwrap();
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
