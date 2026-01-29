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
        instructions::{Instruction, InstructionFlags},
        operand::{Name, Operand},
    },
    types::Typeref,
};

/// Overflow policies for integer operations
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum OverflowPolicy {
    /// Wrap around on overflow
    Wrap,
    /// Panic on overflow
    Panic,
    /// Saturate to the maximum or minimum value on overflow
    /// (Note: Saturation behavior may vary based on the operation)
    Saturate,
}

/// Additional signedness policy for overflow handling
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum OverflowSignednessPolicy {
    /// Wrap (signedness does not matter for wrap)
    Wrap,

    /// Signed saturation (two's complement)
    SSat,

    /// Unsigned saturation
    USat,

    /// Signed trap (panic on overflow)
    STrap,

    /// Unsigned trap (panic on overflow)
    UTrap,
}

impl std::str::FromStr for OverflowSignednessPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        OverflowSignednessPolicy::iter()
            .find(|op| op.to_str() == s)
            .ok_or(())
    }
}

impl OverflowSignednessPolicy {
    /// Returns the string representation of the [`OverflowSignednessPolicy`].
    pub fn to_str(&self) -> &'static str {
        match self {
            OverflowSignednessPolicy::Wrap => "wrap",
            OverflowSignednessPolicy::SSat => "ssat",
            OverflowSignednessPolicy::USat => "usat",
            OverflowSignednessPolicy::STrap => "strap",
            OverflowSignednessPolicy::UTrap => "utrap",
        }
    }

    /// Returns associated signedness if applicable
    pub fn signedness(&self) -> Option<IntegerSignedness> {
        match self {
            OverflowSignednessPolicy::SSat | OverflowSignednessPolicy::STrap => {
                Some(IntegerSignedness::Signed)
            }
            OverflowSignednessPolicy::USat | OverflowSignednessPolicy::UTrap => {
                Some(IntegerSignedness::Unsigned)
            }
            OverflowSignednessPolicy::Wrap => None,
        }
    }
}

/// Signedness for integer operations
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum IntegerSignedness {
    Signed,
    Unsigned,
}

impl std::str::FromStr for IntegerSignedness {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        IntegerSignedness::iter()
            .find(|op| op.to_str() == s)
            .ok_or(())
    }
}

impl IntegerSignedness {
    /// Returns the string representation of the [`IntegerSignedness`].
    pub fn to_str(&self) -> &'static str {
        match self {
            IntegerSignedness::Signed => "signed",
            IntegerSignedness::Unsigned => "unsigned",
        }
    }
}

/// Integer comparison operations
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum ICmpVariant {
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

impl std::str::FromStr for ICmpVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ICmpVariant::iter().find(|op| op.to_str() == s).ok_or(())
    }
}

impl ICmpVariant {
    /// Returns the string representation of the [`ICmpOp`].
    pub fn to_str(&self) -> &'static str {
        match self {
            ICmpVariant::Eq => "eq",
            ICmpVariant::Ne => "ne",
            ICmpVariant::Ugt => "ugt",
            ICmpVariant::Uge => "uge",
            ICmpVariant::Ult => "ult",
            ICmpVariant::Ule => "ule",
            ICmpVariant::Sgt => "sgt",
            ICmpVariant::Sge => "sge",
            ICmpVariant::Slt => "slt",
            ICmpVariant::Sle => "sle",
        }
    }

    /// Returns true if the comparison is unsigned
    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            ICmpVariant::Ugt
                | ICmpVariant::Uge
                | ICmpVariant::Ult
                | ICmpVariant::Ule
                | ICmpVariant::Eq
                | ICmpVariant::Ne
        )
    }

    /// Returns true if the comparison is signed
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            ICmpVariant::Sgt
                | ICmpVariant::Sge
                | ICmpVariant::Slt
                | ICmpVariant::Sle
                | ICmpVariant::Eq
                | ICmpVariant::Ne
        )
    }
}

/// Integer shift operations disambiguation
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum IShiftVariant {
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

impl std::str::FromStr for IShiftVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        IShiftVariant::iter().find(|op| op.to_str() == s).ok_or(())
    }
}

impl IShiftVariant {
    /// Returns the string representation of the [`IShiftOp`].
    pub fn to_str(&self) -> &'static str {
        match self {
            IShiftVariant::Lsl => "lsl",
            IShiftVariant::Lsr => "lsr",
            IShiftVariant::Asr => "asr",
            IShiftVariant::Rol => "rol",
            IShiftVariant::Ror => "ror",
        }
    }
}

/// Integer addition instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IAdd {
    /// Destination SSA name receiving the sum.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
    /// Overflow handling policy.
    pub variant: OverflowSignednessPolicy,
}

impl Instruction for IAdd {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer substraction instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct ISub {
    /// Destination SSA name receiving the difference.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
    /// Overflow handling policy.
    pub variant: OverflowSignednessPolicy,
}

impl Instruction for ISub {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer multiplication instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IMul {
    /// Destination SSA name receiving the product.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
    /// Overflow handling policy.
    pub variant: OverflowSignednessPolicy,
}

impl Instruction for IMul {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer division instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IDiv {
    /// Destination SSA name receiving the quotient.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
    /// Signedness governing division semantics.
    pub signedness: IntegerSignedness,
}

impl Instruction for IDiv {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer remainder instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IRem {
    /// Destination SSA name receiving the remainder.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
    /// Signedness governing remainder semantics.
    pub signedness: IntegerSignedness,
}

impl Instruction for IRem {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer comparison instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct ICmp {
    /// Destination SSA name receiving the predicate result.
    pub dest: Name,

    /// Must be [`crate::types::primary::IType::I1`] if operands are fp, otherwise if operands
    /// are vector of fp(s), must be vector of [`crate::types::primary::IType::I1`] of same length.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
    /// Comparison predicate.
    pub variant: ICmpVariant,
}

impl Instruction for ICmp {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer shift instruction
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct ISht {
    /// Destination SSA name receiving the shifted value.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Value to shift.
    pub lhs: Operand,
    /// Shift amount.
    pub rhs: Operand,
    /// Shift variant (logical, arithmetic, rotate, etc.).
    pub variant: IShiftVariant,
}

impl Instruction for ISht {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer negation instruction
/// (Negates the value of the operand)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct INeg {
    /// Destination SSA name receiving the negated value.
    pub dest: Name,
    /// Integer type for the operand and result.
    pub ty: Typeref,
    /// Operand to negate.
    pub value: Operand,
}

impl Instruction for INeg {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer bitwise NOT instruction
/// (Flips all bits of the operand)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct INot {
    /// Destination SSA name receiving the inverted value.
    pub dest: Name,
    /// Integer type for the operand and result.
    pub ty: Typeref,
    /// Operand to invert.
    pub value: Operand,
}

impl Instruction for INot {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer AND instruction (bitwise AND, logical is equivalent when working on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IAnd {
    /// Destination SSA name receiving the bitwise conjunction.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for IAnd {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer OR instruction (bitwise OR, logical is equivalent when working on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IOr {
    /// Destination SSA name receiving the bitwise disjunction.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for IOr {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Integer XOR instruction (bitwise XOR, logical is equivalent when working on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IXor {
    /// Destination SSA name receiving the bitwise exclusive-or.
    pub dest: Name,
    /// Integer type shared by the operands and result.
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for IXor {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Implies instruction (logical implication, works on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IImplies {
    /// Destination SSA name receiving the implication result.
    pub dest: Name,
    /// Integer type shared by the operands and result (normally i1).
    pub ty: Typeref,
    /// Antecedent operand (if part).
    pub lhs: Operand,
    /// Consequent operand (then part).
    pub rhs: Operand,
}

impl Instruction for IImplies {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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

/// Equivalence instruction (logical equivalence, works on type i1)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct IEquiv {
    /// Destination SSA name receiving the equivalence result.
    pub dest: Name,
    /// Integer type shared by the operands and result (normally i1).
    pub ty: Typeref,
    /// Left-hand operand.
    pub lhs: Operand,
    /// Right-hand operand.
    pub rhs: Operand,
}

impl Instruction for IEquiv {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE | InstructionFlags::ARITHMETIC_INT
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
