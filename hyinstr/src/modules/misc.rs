use crate::{
    modules::{
        CallingConvention, Instruction,
        operand::{Name, Operand},
    },
    types::Typeref,
};

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
