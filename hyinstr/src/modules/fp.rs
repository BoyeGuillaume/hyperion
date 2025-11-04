//! Floating‑point instructions
//!
//! IEEE‑754 oriented arithmetic operations and comparisons. Each instruction
//! specifies its destination `Name`, the floating‑point `Typeref`, and input
//! operands.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    modules::{
        Instruction,
        operand::{Name, Operand},
    },
    types::Typeref,
};

/// Floating-point comparison operations
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FCmpOp {
    /// Ordered and equal (i.e., neither operand is NaN and lhs == rhs)
    Oeq,
    /// Ordered and greater than (i.e., neither operand is NaN and lhs > rhs)
    Ogt,
    /// Ordered and greater than or equal (i.e., neither operand is NaN and lhs >= rhs)
    Oge,
    /// Ordered and less than (i.e., neither operand is NaN and lhs < rhs)
    Olt,
    /// Ordered and less than or equal (i.e., neither operand is NaN and lhs <= rhs)
    Ole,
    /// Ordered and not equal (i.e., neither operand is NaN and lhs != rhs)
    One,
    /// Unordered or equal (i.e., at least one operand is NaN or lhs == rhs)
    Ueq,
    /// Unordered or greater than (i.e., at least one operand is NaN or lhs > rhs)
    Ugt,
    /// Unordered or greater than or equal (i.e., at least one operand is NaN or lhs >= rhs)
    Uge,
    /// Unordered or less than (i.e., at least one operand is NaN or lhs < rhs)
    Ult,
    /// Unordered or less than or equal (i.e., at least one operand is NaN or lhs <= rhs)
    Ule,
    /// Unordered or not equal (i.e., at least one operand is NaN or lhs != rhs)
    Une,
    /// Ordered (i.e., neither operand is NaN)
    Ord,
}

/// Floating-point addition instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FAdd {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for FAdd {
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

/// Floating-point subtraction instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FSub {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for FSub {
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

/// Floating-point multiplication instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FMul {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for FMul {
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

/// Floating-point division instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FDiv {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for FDiv {
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

/// Floating-point remainder instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FRem {
    pub dest: Name,
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
}

impl Instruction for FRem {
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

/// Floating-point negation instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FNeg {
    pub dest: Name,
    pub ty: Typeref,
    pub value: Operand,
}

impl Instruction for FNeg {
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

/// Floating-point comparison instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FCmp {
    pub dest: Name,
    /// Must be [`crate::types::primary::IType::I1`] if operands are fp, otherwise if operands
    /// are vector of fp(s), must be vector of [`crate::types::primary::IType::I1`] of same length.
    pub ty: Typeref,
    pub lhs: Operand,
    pub rhs: Operand,
    pub op: FCmpOp,
}

impl Instruction for FCmp {
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
