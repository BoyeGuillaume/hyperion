use num_bigint::BigInt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::primary::IType;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub struct IConst {
    pub ty: IType,
    pub value: BigInt,
}

impl From<u8> for IConst {
    fn from(value: u8) -> Self {
        Self {
            ty: IType::I8,
            value: value.into(),
        }
    }
}

impl From<u16> for IConst {
    fn from(value: u16) -> Self {
        Self {
            ty: IType::I16,
            value: value.into(),
        }
    }
}

impl From<u32> for IConst {
    fn from(value: u32) -> Self {
        Self {
            ty: IType::I32,
            value: value.into(),
        }
    }
}

impl From<u64> for IConst {
    fn from(value: u64) -> Self {
        Self {
            ty: IType::I64,
            value: value.into(),
        }
    }
}
