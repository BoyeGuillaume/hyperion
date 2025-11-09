use crate::{
    modules::{
        CallingConvention, Instruction,
        operand::{Label, Name, Operand},
    },
    types::Typeref,
};

/// Assertion instruction
///
/// This is a meta-instruction used for verification purposes. It should never
/// appear in executable code. It should point to a condition that must hold at
/// this program point. Therefore `assert %cond` signifies that `%cond` IS true
/// and is similar to the statement `%cond == true`. Proof for assertions can
/// be provided by the derivation engine or external tools.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Assert {
    /// The condition to assert. This should evaluate to a boolean value.
    pub condition: Operand,
}

impl Instruction for Assert {
    fn is_meta_instruction(&self) -> bool {
        true
    }

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

/// Free variable instruction
///
/// A special instruction used to declare free variables within a function. This is a
/// meta-instruction and should NEVER appear in executable code. Free variables are
/// symbolic placeholders that are used for proof generation and verification purposes.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FreeVar {
    /// The destination SSA name for the free variable.
    pub dest: Name,
    /// The type of the free variable.
    pub ty: Typeref,
}

impl Instruction for FreeVar {
    fn is_meta_instruction(&self) -> bool {
        true
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::empty()
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::empty()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Assumption instruction
///
/// Assumptions are similar to assertions, but they indicate conditions that are
/// expected to hold true at a specific program point.
/// They are mostly use in cunjunction with [`FreeVar`] instructions to
/// introduce constraints on free variables.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Assume {
    /// The condition to assume. This should evaluate to a boolean value.
    pub condition: Operand,
}

impl Instruction for Assume {
    fn is_meta_instruction(&self) -> bool {
        true
    }

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

/// Phi instruction
///
/// This instruction selects a value based on control flow. It is used to merge
/// values coming from different basic blocks. It should always be placed at the
/// beginning of a basic block.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Phi {
    /// The destination SSA name for the result of the phi instruction.
    pub dest: Name,

    /// The type of the value being selected.
    pub ty: Typeref,

    /// The incoming values and their corresponding predecessor basic blocks.
    pub values: Vec<(Label, Operand)>, // (predecessor block label, value name)
}

impl Instruction for Phi {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        self.values.iter().map(|(_, op)| op)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        self.values.iter_mut().map(|(_, op)| op)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Select instruction
///
/// This instruction selects one of two values based on a condition.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Select {
    /// The destination SSA name for the result of the select instruction.
    pub dest: Name,
    /// The condition operand. Should evaluate to a boolean value.
    pub condition: Operand,
    /// The operand to select if the condition is true.
    pub true_value: Operand,
    /// The operand to select if the condition is false.
    pub false_value: Operand,
    /// The type of the values being selected.
    pub ty: Typeref,
}

impl Instruction for Select {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.condition)
            .chain(std::iter::once(&self.true_value))
            .chain(std::iter::once(&self.false_value))
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.condition)
            .chain(std::iter::once(&mut self.true_value))
            .chain(std::iter::once(&mut self.false_value))
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}
