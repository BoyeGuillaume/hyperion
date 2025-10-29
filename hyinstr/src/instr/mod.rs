#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumTryAs};
pub mod fp;
pub mod int;
pub mod mem;
pub mod operand;

pub type Reg = usize;

use crate::instr::{fp::*, int::*, mem::*};

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
