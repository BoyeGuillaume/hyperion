//! Floating‑point instructions
//!
//! IEEE‑754 oriented arithmetic operations and comparisons. Each instruction
//! specifies its destination `Name`, the floating‑point `Typeref`, and input
//! operands.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use strum::IntoEnumIterator;

use crate::modules::instructions::Instruction;
use crate::modules::instructions::InstructionFlags;
use crate::{
    modules::operand::{Name, Operand},
    types::Typeref,
};

/// Floating-point comparison operations
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum FCmpVariant {
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

impl std::str::FromStr for FCmpVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        FCmpVariant::iter().find(|op| op.to_str() == s).ok_or(())
    }
}

impl FCmpVariant {
    /// Returns the string representation of the [`FCmpOp`].
    pub fn to_str(&self) -> &'static str {
        match self {
            FCmpVariant::Oeq => "oeq",
            FCmpVariant::Ogt => "ogt",
            FCmpVariant::Oge => "oge",
            FCmpVariant::Olt => "olt",
            FCmpVariant::Ole => "ole",
            FCmpVariant::One => "one",
            FCmpVariant::Ueq => "ueq",
            FCmpVariant::Ugt => "ugt",
            FCmpVariant::Uge => "uge",
            FCmpVariant::Ult => "ult",
            FCmpVariant::Ule => "ule",
            FCmpVariant::Une => "une",
            FCmpVariant::Ord => "ord",
        }
    }
}

/// Floating-point addition instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct FAdd {
    /// Destination SSA name receiving the sum.
    pub dest: Name,
    /// Floating-point type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for FAdd {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_FP
    }

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

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Floating-point subtraction instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct FSub {
    /// Destination SSA name receiving the difference.
    pub dest: Name,
    /// Floating-point type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for FSub {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_FP
    }

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

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Floating-point multiplication instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct FMul {
    /// Destination SSA name receiving the product.
    pub dest: Name,
    /// Floating-point type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for FMul {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_FP
    }

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

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Floating-point division instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct FDiv {
    /// Destination SSA name receiving the quotient.
    pub dest: Name,
    /// Floating-point type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for FDiv {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_FP
    }

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

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Floating-point remainder instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct FRem {
    /// Destination SSA name receiving the remainder.
    pub dest: Name,
    /// Floating-point type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for FRem {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_FP
    }

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

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Floating-point negation instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct FNeg {
    /// Destination SSA name receiving the negated value.
    pub dest: Name,
    /// Floating-point type shared by the operand and result.
    pub ty: Typeref,
    /// Operand to negate.
    pub value: Operand,
}

impl Instruction for FNeg {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_FP
    }

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

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Floating-point comparison instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct FCmp {
    /// Destination SSA name receiving the comparison result.
    pub dest: Name,
    /// Must be [`crate::types::primary::IType::I1`] if operands are fp, otherwise if operands
    /// are vector of fp(s), must be vector of [`crate::types::primary::IType::I1`] of same length.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
    /// Comparison predicate.
    pub variant: FCmpVariant,
}

impl Instruction for FCmp {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_FP
    }

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

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}
