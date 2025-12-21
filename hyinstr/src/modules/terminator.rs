//! Module definitions for control flow instructions.
//!
//! Branching and flow control operations, including conditional
//! branches, jumps, and function calls. Each instruction specifies its
//! target labels and input operands as needed.
use auto_enums::auto_enum;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumIter, EnumTryAs, IntoEnumIterator};

use crate::modules::operand::{Label, Name, Operand};

/// Conditional branch instruction
///
/// See `Label` in `operand.rs` for more information about code labels.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Branch {
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
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumTryAs, EnumIs, EnumDiscriminants)]
#[strum_discriminants(name(HyTerminatorOp))]
#[strum_discriminants(derive(EnumIter))]
#[cfg_attr(feature = "serde", strum_discriminants(derive(Serialize, Deserialize)))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HyTerminator {
    Branch(Branch),
    Jump(Jump),
    Ret(Ret),
    Trap(Trap),
}

impl HyTerminatorOp {
    /// Return the canonical mnemonic used when printing the terminator.
    pub fn opname(&self) -> &'static str {
        match self {
            HyTerminatorOp::Branch => "branch",
            HyTerminatorOp::Jump => "jump",
            HyTerminatorOp::Ret => "ret",
            HyTerminatorOp::Trap => "trap",
        }
    }

    /// Parse a mnemonic into its corresponding terminator kind.
    pub fn from_str(s: &str) -> Option<Self> {
        HyTerminatorOp::iter().find(|op| op.opname() == s)
    }
}

impl HyTerminator {
    /// Return the discriminant for this terminator value.
    pub fn op(&self) -> HyTerminatorOp {
        self.into()
    }
}

impl HyTerminator {
    /// Iterate over operands consumed by this terminator.
    #[auto_enum(Iterator)]
    pub fn operands(&self) -> impl Iterator<Item = &Operand> {
        match self {
            HyTerminator::Branch(cbranch) => std::iter::once(&cbranch.cond),
            HyTerminator::Jump(_) => std::iter::empty(),
            HyTerminator::Ret(ret) => ret.value.iter(),
            HyTerminator::Trap(_) => std::iter::empty(),
        }
    }

    /// Iterate over SSA dependencies referenced by this terminator.
    pub fn dependencies(&self) -> impl Iterator<Item = Name> {
        self.operands().filter_map(|op| {
            if let Operand::Reg(name) = op {
                Some(*name)
            } else {
                None
            }
        })
    }

    /// Iterate mutably over operands consumed by this terminator.
    #[auto_enum(Iterator)]
    pub fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        match self {
            HyTerminator::Branch(cbranch) => std::iter::once(&mut cbranch.cond),
            HyTerminator::Jump(_) => std::iter::empty(),
            HyTerminator::Ret(ret) => ret.value.iter_mut(),
            HyTerminator::Trap(_) => std::iter::empty(),
        }
    }

    /// Iterate mutably over SSA dependencies referenced by this terminator.
    pub fn dependencies_mut(&mut self) -> impl Iterator<Item = &mut Name> {
        self.operands_mut().filter_map(|op| {
            if let Operand::Reg(name) = op {
                Some(name)
            } else {
                None
            }
        })
    }

    /// Iterate over branch targets along with the optional condition operand.
    #[auto_enum(Iterator)]
    pub fn iter_targets(&self) -> impl Iterator<Item = (Label, Option<&'_ Operand>)> + '_ {
        match self {
            HyTerminator::Branch(cbranch) => [
                (cbranch.target_true, Some(&cbranch.cond)),
                (cbranch.target_false, None),
            ]
            .into_iter(),
            HyTerminator::Jump(jump) => [(jump.target, None)].into_iter(),
            HyTerminator::Ret(_) => std::iter::empty(),
            HyTerminator::Trap(_) => std::iter::empty(),
        }
    }
}

macro_rules! define_terminator_from {
    ($typ:ty, $variant:ident) => {
        impl From<$typ> for HyTerminator {
            fn from(inst: $typ) -> Self {
                HyTerminator::$variant(inst)
            }
        }
    };
}

define_terminator_from!(Branch, Branch);
define_terminator_from!(Jump, Jump);
define_terminator_from!(Ret, Ret);
define_terminator_from!(Trap, Trap);
