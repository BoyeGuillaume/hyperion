//! Integer instructions
//!
//! Arithmetic, comparisons, shifts, and bitwise operations over integer
//! values. Each instruction carries its destination `Name`, an `Typeref`, and
//! its input operands. Overflow and signedness where relevant are explicit
//! parameters of the instruction.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    modules::{
        Instruction,
        operand::{Name, Operand},
    },
    types::Typeref,
};

/// Overflow policies for integer operations
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OverflowPolicy {
    /// Wrap around on overflow
    Wrap,
    /// Panic on overflow
    Panic,
    /// Saturate to the maximum or minimum value on overflow
    /// (Note: Saturation behavior may vary based on the operation)
    Saturate,
}

/// Signedness for integer operations
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IntegerSignedness {
    Signed,
    Unsigned,
}

/// Integer comparison operations
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ICmpOp {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Unsigned greater than
    Ugt,
    /// Unsigned greater than or equal
    Uge,
    /// Unsigned less than
    Ult,
    /// Unsigned less than or equal
    Ule,
    /// Signed greater than
    Sgt,
    /// Signed greater than or equal
    Sge,
    /// Signed less than
    Slt,
    /// Signed less than or equal
    Sle,
}

impl ICmpOp {
    /// Creates an [`ICmpOp`] from its string representation.
    pub fn from_str(s: &str) -> Option<Self> {
        ICmpOp::iter().find(|op| op.to_str() == s)
    }

    /// Returns the string representation of the [`ICmpOp`].
    pub fn to_str(&self) -> &'static str {
        match self {
            ICmpOp::Eq => "eq",
            ICmpOp::Ne => "ne",
            ICmpOp::Ugt => "ugt",
            ICmpOp::Uge => "uge",
            ICmpOp::Ult => "ult",
            ICmpOp::Ule => "ule",
            ICmpOp::Sgt => "sgt",
            ICmpOp::Sge => "sge",
            ICmpOp::Slt => "slt",
            ICmpOp::Sle => "sle",
        }
    }

    /// Returns true if the comparison is unsigned
    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            ICmpOp::Ugt | ICmpOp::Uge | ICmpOp::Ult | ICmpOp::Ule | ICmpOp::Eq | ICmpOp::Ne
        )
    }

    /// Returns true if the comparison is signed
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            ICmpOp::Sgt | ICmpOp::Sge | ICmpOp::Slt | ICmpOp::Sle | ICmpOp::Eq | ICmpOp::Ne
        )
    }
}

/// Integer shift operations disambiguation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IShiftOp {
    /// Logical left shift
    Lsl,
    /// Logical right shift
    Lsr,
    /// Arithmetic right shift
    Asr,
    /// Rotate left
    Rol,
    /// Rotate right
    Ror,
}

/// Integer addition instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IAdd {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
    pub overflow: OverflowPolicy,
    pub signedness: IntegerSignedness,
}

impl Instruction for IAdd {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Integer substraction instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ISub {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
    pub overflow: OverflowPolicy,
    pub signedness: IntegerSignedness,
}

impl Instruction for ISub {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Integer multiplication instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IMul {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
    pub overflow: OverflowPolicy,
    pub signedness: IntegerSignedness,
}

impl Instruction for IMul {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Integer division instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IDiv {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
    pub signedness: IntegerSignedness,
}

impl Instruction for IDiv {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Integer remainder instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IRem {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
    pub signedness: IntegerSignedness,
}

impl Instruction for IRem {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Integer comparison instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ICmp {
    pub dest: Name,

    /// Must be [`crate::types::primary::IType::I1`] if operands are fp, otherwise if operands
    /// are vector of fp(s), must be vector of [`crate::types::primary::IType::I1`] of same length.
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
    pub op: ICmpOp,
}

impl Instruction for ICmp {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Integer shift instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ISht {
    pub dest: Name,
    pub ty: Typeref,
    pub value: Operand,
    pub shift: Operand,
    pub variant: IShiftOp,
}

impl Instruction for ISht {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.value, &self.shift].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.value, &mut self.shift].into_iter()
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

/// Integer negation instruction
/// (Negates the value of the operand)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct INeg {
    pub dest: Name,
    pub ty: Typeref,
    pub value: Operand,
}

impl Instruction for INeg {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.value)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.value)
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

/// Integer bitwise NOT instruction
/// (Flips all bits of the operand)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct INot {
    pub dest: Name,
    pub ty: Typeref,
    pub value: Operand,
}

impl Instruction for INot {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.value)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.value)
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

/// Integer AND instruction (bitwise AND, logical is equivalent when working on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IAnd {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for IAnd {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Integer OR instruction (bitwise OR, logical is equivalent when working on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IOr {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for IOr {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Integer XOR instruction (bitwise XOR, logical is equivalent when working on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IXor {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for IXor {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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

/// Implies instruction (logical implication, works on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IImplies {
    pub dest: Name,
    pub ty: Typeref,
    pub premise: Operand,
    pub conclusion: Operand,
}

impl Instruction for IImplies {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.premise, &self.conclusion].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.premise, &mut self.conclusion].into_iter()
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

/// Equivalence instruction (logical equivalence, works on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IEquiv {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for IEquiv {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.lhs, &self.rhs].into_iter()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.lhs, &mut self.rhs].into_iter()
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
