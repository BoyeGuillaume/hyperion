//! Module definitions for control flow instructions.
//!
//! Branching and flow control operations, including conditional
//! branches, jumps, and function calls. Each instruction specifies its
//! target labels and input operands as needed.
use auto_enums::auto_enum;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumIter, EnumTryAs, IntoEnumIterator};

use crate::{
    modules::{
        instructions::{Instruction, InstructionFlags},
        operand::{Label, Name, Operand},
    },
    types::Typeref,
};

/// Common interface for terminator instructions
///
/// A terminator instruction is one that alters the control flow of execution,
/// typically by transferring control to one or more target labels. Examples
/// include branches, jumps, and returns.
pub trait Terminator: Instruction {
    fn iter_targets(&self) -> impl Iterator<Item = (Label, Option<&Operand>)>;
}

/// Conditional branch instruction
///
/// See `Label` in `operand.rs` for more information about code labels.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
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

impl Instruction for Branch {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::TERMINATOR
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.cond)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.cond)
    }

    fn destination(&self) -> Option<Name> {
        None
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::empty()
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::empty()
    }

    fn destination_type(&self) -> Option<Typeref> {
        None
    }
}

impl Terminator for Branch {
    fn iter_targets(&self) -> impl Iterator<Item = (Label, Option<&Operand>)> {
        [
            (self.target_true, Some(&self.cond)),
            (self.target_false, None),
        ]
        .into_iter()
    }
}

/// Unconditional jump instruction
///
/// See `Label` in `operand.rs` for more information about code labels.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Jump {
    /// The label to jump to.
    pub target: Label,
}

impl Instruction for Jump {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::TERMINATOR
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::empty()
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::empty()
    }

    fn destination(&self) -> Option<Name> {
        None
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::empty()
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::empty()
    }

    fn destination_type(&self) -> Option<Typeref> {
        None
    }
}

impl Terminator for Jump {
    fn iter_targets(&self) -> impl Iterator<Item = (Label, Option<&Operand>)> {
        std::iter::once((self.target, None))
    }
}

/// Return from function instruction. Optionally returns a value.
///
/// If `value` is `None`, it indicates a `void` return.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Ret {
    pub value: Option<Operand>,
}

impl Instruction for Ret {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::TERMINATOR
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        self.value.iter()
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        self.value.iter_mut()
    }

    fn destination(&self) -> Option<Name> {
        None
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::empty()
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::empty()
    }

    fn destination_type(&self) -> Option<Typeref> {
        None
    }
}

impl Terminator for Ret {
    fn iter_targets(&self) -> impl Iterator<Item = (Label, Option<&Operand>)> {
        std::iter::empty()
    }
}

/// Trap instruction to indicate an unrecoverable error or exceptional condition.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Trap;

impl Instruction for Trap {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::TERMINATOR
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::empty()
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::empty()
    }

    fn destination(&self) -> Option<Name> {
        None
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::empty()
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::empty()
    }

    fn destination_type(&self) -> Option<Typeref> {
        None
    }
}

impl Terminator for Trap {
    fn iter_targets(&self) -> impl Iterator<Item = (Label, Option<&Operand>)> {
        std::iter::empty()
    }
}

/// Control flow terminator instructions
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumTryAs, EnumIs, EnumDiscriminants)]
#[strum_discriminants(name(HyTerminatorOp))]
#[strum_discriminants(derive(EnumIter))]
#[cfg_attr(feature = "serde", strum_discriminants(derive(Serialize, Deserialize)))]
#[cfg_attr(
    feature = "borsh",
    strum_discriminants(derive(borsh::BorshSerialize, borsh::BorshDeserialize))
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
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
}
impl std::str::FromStr for HyTerminatorOp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        HyTerminatorOp::iter().find(|op| op.opname() == s).ok_or(())
    }
}

impl HyTerminator {
    /// Return the discriminant for this terminator value.
    pub fn op(&self) -> HyTerminatorOp {
        self.into()
    }
}

impl HyTerminator {
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

macro_rules! define_instr_any_instr {
    (
        $($variant:ident),* $(,)?
    ) => {
        impl Instruction for HyTerminator {
            fn flags(&self) -> InstructionFlags {
                match self {
                    $(HyTerminator::$variant(inst) => inst.flags(),)*
                }
            }

            #[auto_enum(Iterator)]
            fn operands(&self) -> impl Iterator<Item = &Operand> {
                match self {
                    $(HyTerminator::$variant(inst) => inst.operands(),)*
                }
            }

            #[auto_enum(Iterator)]
            fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
                match self {
                    $(HyTerminator::$variant(instr) => instr.operands_mut(),)*
                }
            }

            fn destination(&self) -> Option<Name> {
                match self {
                    $(HyTerminator::$variant(instr) => instr.destination(),)*
                }
            }

            #[auto_enum(Iterator)]
            fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
                match self {
                    $(HyTerminator::$variant(instr) => instr.referenced_types(),)*
                }
            }

            #[auto_enum(Iterator)]
            fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
                match self {
                    $(HyTerminator::$variant(instr) => instr.referenced_types_mut(),)*
                }
            }

            fn destination_type(&self) -> Option<Typeref> {
                match self {
                    $(HyTerminator::$variant(instr) => instr.destination_type(),)*
                }
            }
        }

        impl Terminator for HyTerminator {
            #[auto_enum(Iterator)]
            fn iter_targets(&self) -> impl Iterator<Item = (Label, Option<&Operand>)> {
                match self {
                    $(HyTerminator::$variant(inst) => inst.iter_targets(),)*
                }
            }
        }
    };
}

define_instr_any_instr! {
    Branch,
    Jump,
    Ret,
    Trap,
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
