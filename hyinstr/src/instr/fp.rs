use crate::{constants::ConstFp, instr::Reg, name::Name};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

#[derive(Debug, Clone, Copy, PartialEq, EnumTryAs, EnumIs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FOp {
    Reg(Reg),
    Imm(ConstFp),
}

/// Floating-point comparison operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FAdd {
    pub dst: Name,
    pub lhs: FOp,
    pub rhs: FOp,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FSub {
    pub dst: Name,
    pub lhs: FOp,
    pub rhs: FOp,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FMul {
    pub dst: Name,
    pub lhs: FOp,
    pub rhs: FOp,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FDiv {
    pub dst: Name,
    pub lhs: FOp,
    pub rhs: FOp,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FRem {
    pub dst: Name,
    pub lhs: FOp,
    pub rhs: FOp,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FNeg {
    pub dst: Name,
    pub val: FOp,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FCmp {
    pub dst: Name,
    pub lhs: FOp,
    pub rhs: FOp,
    pub op: FCmpOp,
}
