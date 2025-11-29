//! Module definitions for control flow instructions.
//!
//! Branching and flow control operations, including conditional
//! branches, jumps, and function calls. Each instruction specifies its
//! target labels and input operands as needed.
use auto_enums::auto_enum;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::modules::{
    Module,
    operand::{Label, Name, Operand},
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

/// Return from function instruction. Optionally returns a value.
///
/// If `value` is `None`, it indicates a `void` return.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Ret {
    pub value: Option<Operand>,
}

/// Trap instruction to indicate an unrecoverable error or exceptional condition.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Trap;

/// Control flow terminator instructions
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Terminator {
    CBranch(CBranch),
    Jump(Jump),
    Ret(Ret),
    Trap(Trap),
}

impl Terminator {
    pub fn fmt<'a>(&'a self, module: Option<&'a Module>) -> impl std::fmt::Display + 'a {
        struct Fmt<'a> {
            terminator: &'a Terminator,
            module: Option<&'a Module>,
        }

        impl std::fmt::Display for Fmt<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.terminator {
                    Terminator::CBranch(cbranch) => write!(
                        f,
                        "branch {}, {:#}, {:#}",
                        cbranch.cond.fmt(self.module),
                        cbranch.target_true,
                        cbranch.target_false
                    ),
                    Terminator::Jump(jump) => {
                        write!(f, "jump {}", jump.target)
                    }
                    Terminator::Ret(ret) => {
                        if let Some(value) = &ret.value {
                            write!(f, "ret {:#}", value.fmt(self.module))
                        } else {
                            write!(f, "ret void")
                        }
                    }
                    Terminator::Trap(_) => {
                        write!(f, "trap")
                    }
                }
            }
        }

        Fmt {
            terminator: self,
            module,
        }
    }

    #[auto_enum(Iterator)]
    pub fn operands(&self) -> impl Iterator<Item = &Operand> {
        match self {
            Terminator::CBranch(cbranch) => std::iter::once(&cbranch.cond),
            Terminator::Jump(_) => std::iter::empty(),
            Terminator::Ret(ret) => ret.value.iter(),
            Terminator::Trap(_) => std::iter::empty(),
        }
    }

    pub fn dependencies(&self) -> impl Iterator<Item = Name> {
        self.operands().filter_map(|op| {
            if let Operand::Reg(name) = op {
                Some(*name)
            } else {
                None
            }
        })
    }

    #[auto_enum(Iterator)]
    pub fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        match self {
            Terminator::CBranch(cbranch) => std::iter::once(&mut cbranch.cond),
            Terminator::Jump(_) => std::iter::empty(),
            Terminator::Ret(ret) => ret.value.iter_mut(),
            Terminator::Trap(_) => std::iter::empty(),
        }
    }

    pub fn dependencies_mut(&mut self) -> impl Iterator<Item = &mut Name> {
        self.operands_mut().filter_map(|op| {
            if let Operand::Reg(name) = op {
                Some(name)
            } else {
                None
            }
        })
    }

    #[auto_enum(Iterator)]
    pub fn iter_targets(&self) -> impl Iterator<Item = (Label, Option<&'_ Operand>)> + '_ {
        match self {
            Terminator::CBranch(cbranch) => [
                (cbranch.target_true, Some(&cbranch.cond)),
                (cbranch.target_false, None),
            ]
            .into_iter(),
            Terminator::Jump(jump) => [(jump.target, None)].into_iter(),
            Terminator::Ret(_) => std::iter::empty(),
            Terminator::Trap(_) => std::iter::empty(),
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
define_terminator_from!(Ret, Ret);
