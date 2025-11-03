//! Module definitions for control flow instructions.
//!
//! Branching and flow control operations, including conditional
//! branches, jumps, and function calls. Each instruction specifies its
//! target labels and input operands as needed.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    modules::{
        CallingConvention, Module,
        operand::{Label, Name, Operand},
    },
    types::{TypeRegistry, Typeref},
};

/// Conditional branch instruction
///
/// See `Label` in `operand.rs` for more information about code labels.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CBranch {
    /// The condition operand; should evaluate to a boolean value.
    ///
    /// The condition is evaluated, and if it is true (non-zero), control
    /// transfers to `target_true`; otherwise, it transfers to `target_false`.
    pub cond: Operand,
    /// The label to jump to if the condition is true.
    pub target_true: Label,
    /// The label to jump to if the condition is false.
    pub target_false: Label,
}

/// Unconditional jump instruction
///
/// See `Label` in `operand.rs` for more information about code labels.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Jump {
    /// The label to jump to.
    pub target: Label,
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

    /// The label to jump to after the call completes.
    pub exit_label: Label,

    /// This should only be `Some` for calls to external functions (i.e., not
    /// defined within the current module)
    pub cconv: Option<CallingConvention>,
}

/// Return from function instruction. Optionally returns a value.
///
/// If `value` is `None`, it indicates a `void` return.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Ret {
    pub value: Option<Operand>,
}

/// Control flow terminator instructions
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Terminator {
    CBranch(CBranch),
    Jump(Jump),
    Invoke(Invoke),
    Ret(Ret),
}

impl Terminator {
    pub fn fmt<'a>(
        &'a self,
        registry: &'a TypeRegistry,
        module: Option<&'a Module>,
    ) -> impl std::fmt::Display + 'a {
        struct Fmt<'a> {
            terminator: &'a Terminator,
            registry: &'a TypeRegistry,
            module: Option<&'a Module>,
        }

        impl std::fmt::Display for Fmt<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.terminator {
                    Terminator::CBranch(cbranch) => write!(
                        f,
                        "cbranch {} , %{} , %{}",
                        cbranch.cond.fmt(self.module),
                        cbranch.target_true.0,
                        cbranch.target_false.0
                    ),
                    Terminator::Jump(jump) => {
                        write!(f, "jump %{}", jump.target.0)
                    }
                    Terminator::Invoke(invoke) => {
                        let args_str = invoke
                            .args
                            .iter()
                            .map(|arg| arg.fmt(self.module).to_string())
                            .collect::<Vec<_>>()
                            .join(", ");

                        let name_str = invoke.function.fmt(self.module).to_string();

                        if let Some(dest) = invoke.dest {
                            if let Some(ret_ty) = &invoke.ty {
                                write!(
                                    f,
                                    "%{} = invoke {} {} ({}) -> {}",
                                    dest,
                                    self.registry.fmt(*ret_ty),
                                    name_str,
                                    args_str,
                                    invoke.exit_label.0
                                )
                            } else {
                                write!(f, "invoke void({}) -> %{}", args_str, invoke.exit_label.0)
                            }
                        } else {
                            write!(f, "invoke void({}) -> %{}", args_str, invoke.exit_label.0)
                        }
                    }
                    Terminator::Ret(ret) => {
                        if let Some(value) = &ret.value {
                            write!(f, "ret {}", value.fmt(self.module))
                        } else {
                            write!(f, "ret void")
                        }
                    }
                }
            }
        }

        Fmt {
            terminator: self,
            registry,
            module,
        }
    }
}

macro_rules! define_terminator_from {
    ($typ:ty, $variant:ident) => {
        impl From<$typ> for Terminator {
            fn from(inst: $typ) -> Self {
                Terminator::$variant(inst)
            }
        }
    };
}

define_terminator_from!(CBranch, CBranch);
define_terminator_from!(Jump, Jump);
define_terminator_from!(Invoke, Invoke);
define_terminator_from!(Ret, Ret);
