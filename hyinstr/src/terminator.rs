#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TRet {
    pub operand: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TBr {
    pub target: (),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TCondBr {
    pub condition: usize,
    pub true_target: (),
    pub false_target: (),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TSwitch {
    pub operand: usize,
    pub default_target: (),
    pub targets: Vec<(i64, ())>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TInvoke {
    pub function: (),
    pub arguments: Vec<usize>,
    pub result: Option<usize>,
    pub return_target: (),
    pub exception_target: (),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TUnreachable;

#[derive(Debug, Clone, PartialEq, Eq, EnumIs, EnumTryAs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HyTerminator {
    Ret(TRet),
    Br(TBr),
    CondBr(TCondBr),
    Switch(TSwitch),
    Invoke(TInvoke),
    Unreachable(TUnreachable),
}
