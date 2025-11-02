//! Module definitions for control flow instructions.
//!
//! Branching and flow control operations, including conditional
//! branches, jumps, and function calls. Each instruction specifies its
//! target labels and input operands as needed.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    modules::{
        CallingConvention,
        operand::{Label, Name, Operand},
    },
    types::Typeref,
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
