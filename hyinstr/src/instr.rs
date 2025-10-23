#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumTryAs, EnumIs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Constant {
    Int(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumTryAs, EnumIs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IOperand {
    Reg(usize),
    Imm(Constant),
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
pub enum CompareOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
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
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub overflow_policy: IntegerOverflowPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ISub {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub overflow_policy: IntegerOverflowPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IMul {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub overflow_policy: IntegerOverflowPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IDiv {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub signedness: Signedness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IRem {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub signedness: Signedness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IAnd {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IOr {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IXor {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ISht {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub shift_type: ShiftType,
    pub direction: ShiftDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FAdd {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FSub {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FMul {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FDiv {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FRem {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FNeg {
    pub dst: usize,
    pub val: IOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Load {
    pub dst: usize,
    pub addr: (),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Store {
    pub src: usize,
    pub addr: (),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ICmp {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub op: CompareOp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FCmp {
    pub dst: usize,
    pub lhs: IOperand,
    pub rhs: IOperand,
    pub op: CompareOp,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumTryAs, EnumIs)]
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

    Load(Load),
    Store(Store),

    ICmp(ICmp),
    FCmp(FCmp),
}
