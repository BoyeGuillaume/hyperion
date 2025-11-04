use crate::{
    modules::{
        CallingConvention, Instruction,
        operand::{Name, Operand},
    },
    types::Typeref,
};

/// Assertion instruction
///
/// This is (mostly) used for properties derivation and verification. An assertion
/// instruction checks that a given condition holds at runtime; if not, the program
/// should abort or raise an error.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Assert {
    /// The condition to assert. This should evaluate to a boolean value.
    pub condition: Operand,
}

impl Instruction for Assert {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.condition)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.condition)
    }

    fn destination(&self) -> Option<Name> {
        None
    }

    fn set_destination(&mut self, _name: Name) {
        // No destination to set
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::empty()
    }

    fn destination_type(&self) -> Option<Typeref> {
        None
    }
}

/// Function call instruction
///
/// In hyperion, function cannot raise exceptions; thus, it will always jump to
/// the specified `exit_label` after the call completes. In case of errors, either use
/// a return code or never return from the function (e.g., abort).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Invoke {
    /// Should be a reference to a function pointer (either internal or external). We
    /// describe it as an `Operand` to allow dynamic function calls to achieve virtualization
    /// or function pointer tables.
    pub function: Operand,

    /// The argument operands to pass to the function.
    pub args: Vec<Operand>,

    /// The destination SSA name for the return value, if any.
    pub dest: Option<Name>,

    /// The return type of the function being called. `None` for `void` functions.
    pub ty: Option<Typeref>,

    /// This should only be `Some` for calls to external functions (i.e., not
    /// defined within the current module)
    pub cconv: Option<CallingConvention>,
}

impl Instruction for Invoke {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.function).chain(self.args.iter())
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.function).chain(self.args.iter_mut())
    }

    fn destination(&self) -> Option<Name> {
        self.dest
    }

    fn set_destination(&mut self, name: Name) {
        // Cannot change a void return to a non-void return
        if self.dest.is_some() {
            self.dest = Some(name);
        }
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        self.ty.into_iter()
    }

    fn destination_type(&self) -> Option<Typeref> {
        self.ty
    }
}
