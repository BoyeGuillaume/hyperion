#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumTryAs};

use crate::constants::{FpConst, IConst};

pub type Reg = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumTryAs, EnumIs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IOperand {
    Reg(Reg),
    Imm(IConst),
}

#[derive(Debug, Clone, Copy, PartialEq, EnumTryAs, EnumIs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FOperand {
    Reg(Reg),
    Imm(FpConst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IntegerOverflowPolicy {
    Wrap,
    Panic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Signedness {
    Signed,
    Unsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ICompareOp {
    Eq,
    Ne,
    UGt,
    UGe,
    ULt,
    ULe,
    SGt,
    SGe,
    SLt,
    SLe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FpCompareOp {
    Oeq,
    Ogt,
    Oge,
    Olt,
    Ole,
    One,
    Ueq,
    Ugt,
    Uge,
    Ult,
    Ule,
    Une,
    Ord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ShiftType {
    Logical,
    Aritmetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ShiftDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IAdd {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub overflow_policy: IntegerOverflowPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ISub {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub overflow_policy: IntegerOverflowPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IMul {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub overflow_policy: IntegerOverflowPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IDiv {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub signedness: Signedness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IRem {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub signedness: Signedness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IAnd {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IOr {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IXor {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ISht {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub shift_type: ShiftType,
    pub direction: ShiftDirection,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FAdd {
    pub dst: Reg,
    pub lhs: FOperand,
    pub rhs: FOperand,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FSub {
    pub dst: Reg,
    pub lhs: FOperand,
    pub rhs: FOperand,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FMul {
    pub dst: Reg,
    pub lhs: FOperand,
    pub rhs: FOperand,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FDiv {
    pub dst: Reg,
    pub lhs: FOperand,
    pub rhs: FOperand,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FRem {
    pub dst: Reg,
    pub lhs: FOperand,
    pub rhs: FOperand,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FNeg {
    pub dst: Reg,
    pub val: FOperand,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MLoad {
    pub dst: Reg,
    pub addr: (),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MStore {
    pub src: Reg,
    pub addr: (),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ICmp {
    pub dst: Reg,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub op: ICompareOp,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FCmp {
    pub dst: Reg,
    pub lhs: FOperand,
    pub rhs: FOperand,
    pub op: FpCompareOp,
}

#[derive(Debug, Clone, PartialEq, EnumTryAs, EnumIs, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIs))]
#[strum_discriminants(name(HyInstrKind))]
#[cfg_attr(feature = "serde", strum_discriminants(derive(Serialize, Deserialize)))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HyInstr {
    Add(IAdd),
    Sub(ISub),
    Mul(IMul),
    Div(IDiv),
    Rem(IRem),
    Sht(ISht),

    And(IAnd),
    Or(IOr),
    Xor(IXor),

    FAdd(FAdd),
    FSub(FSub),
    FMul(FMul),
    FDiv(FDiv),
    FRem(FRem),
    FNeg(FNeg),

    Load(MLoad),
    Store(MStore),

    ICmp(ICmp),
    FCmp(FCmp),
}
