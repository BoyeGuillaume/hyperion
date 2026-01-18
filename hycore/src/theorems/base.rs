use std::collections::BTreeSet;

use hyinstr::{
    consts::AnyConst,
    modules::{Function, InstructionRef, symbol::FunctionPointer},
};
use uuid::Uuid;

use crate::utils::lazy::{LazyContainer, LazyDirtifierGuard, LazyGuard};

struct TheoremAccelerationStructures {
    /// Collected references to all assert-style meta-instructions found in `function`.
    list_asserts: Vec<InstructionRef>,

    /// Collected references to all assume-style meta-instructions found in `function`.
    list_assumptions: Vec<InstructionRef>,

    /// Collected references to all referenced functions found in `function`.
    list_referenced_functions: BTreeSet<FunctionPointer>,
}

impl TheoremAccelerationStructures {
    /// Scan the theorems function and update [`Theorem::list_asserts`] with all
    /// instructions that represent meta-assertions.
    fn derive_meta_asserts(&mut self, func: &Function) {
        self.list_asserts = func.gather_instructions_by_predicate(|instr| instr.is_meta_assert());
    }

    /// Scan the theorems function and update [`Theorem::list_assumptions`] with all
    /// instructions that represent meta-assumptions (preconditions).
    fn derive_meta_assumptions(&mut self, func: &Function) {
        self.list_assumptions =
            func.gather_instructions_by_predicate(|instr| instr.is_meta_assume());
    }

    /// Scan the function body and populate [`Theorem::list_referenced_functions`] with every
    /// directly referenced function pointer.
    fn derive_referenced_functions(&mut self, func: &Function) {
        self.list_referenced_functions = func
            .iter()
            .filter_map(|(instr, _)| {
                if let Some(call) = instr.try_as_invoke_ref() {
                    use hyinstr::modules::operand::Operand::*;

                    match &call.function {
                        Imm(AnyConst::FuncPtr(func_ptr)) => Some(func_ptr.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect();
    }

    /// Run all derivation steps to populate acceleration structures.
    fn derive(&mut self, func: &Function) {
        self.derive_meta_asserts(func);
        self.derive_meta_assumptions(func);
        self.derive_referenced_functions(func);
    }

    /// Compute func
    fn compute(maybe_self: Option<Self>, func: &Function) -> Self {
        let mut this = match maybe_self {
            Some(s) => s,
            None => Self {
                list_asserts: Vec::new(),
                list_assumptions: Vec::new(),
                list_referenced_functions: BTreeSet::new(),
            },
        };

        this.derive(func);
        this
    }
}

/// Theorems are a set of external properties attached to functions, expressed as meta-functions.
///
/// These meta-functions can contain meta-instructions such as assertions and assumptions,
/// which can be used by provers to verify that the target function adheres to its specification.
///
/// They are external as they do not provide information about the internal workings of the function,
/// only about its external state and behavior.
///
pub struct Theorem {
    /// Uuid (should stay static after creation)
    uuid: Uuid,

    /// The meta-function that carries the specification.
    function: Function,

    /// Acceleration structures derived from the specification function.
    acceleration: LazyContainer<TheoremAccelerationStructures>,
}

impl Theorem {
    /// Unique identifier associated with both the specification and the backing meta-function.
    pub fn uuid(&self) -> Uuid {
        debug_assert!(self.uuid == self.function.uuid);
        self.uuid
    }

    /// Creates a new specification from the given meta-function.
    pub fn new(function: Function) -> Self {
        Self {
            uuid: function.uuid,
            function,
            acceleration: LazyContainer::new(),
        }
    }

    /// Returns the immutable specification function body, recomputing caches if needed.
    pub fn function(&self) -> &Function {
        debug_assert!(self.uuid == self.function.uuid);
        &self.function
    }

    /// Marks acceleration structures as dirty and exposes the mutable function for in-place edits.
    pub fn function_mut(&mut self) -> LazyDirtifierGuard<'_, Function> {
        self.acceleration.dirtify(&mut self.function)
    }

    /// Get a reference to the list of meta-assertion instructions.
    // pub fn list_asserts(&self) -> &[InstructionRef] {}
    pub fn list_asserts(&self) -> LazyGuard<'_, [InstructionRef]> {
        self.acceleration.get(
            |x| TheoremAccelerationStructures::compute(x, &self.function),
            |x| x.list_asserts.as_slice(),
        )
    }

    /// Get a reference to the list of meta-assumption instructions
    pub fn list_assumptions(&self) -> LazyGuard<'_, [InstructionRef]> {
        self.acceleration.get(
            |x| TheoremAccelerationStructures::compute(x, &self.function),
            |x| x.list_assumptions.as_slice(),
        )
    }

    /// Get a reference to the set of directly referenced function pointers.
    pub fn list_referenced_functions(&self) -> LazyGuard<'_, BTreeSet<FunctionPointer>> {
        self.acceleration.get(
            |x| TheoremAccelerationStructures::compute(x, &self.function),
            |x| &x.list_referenced_functions,
        )
    }
}
