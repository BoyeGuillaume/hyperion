use crate::types::primary::FType;
use bigdecimal::{BigDecimal, FromPrimitive};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub struct FConst {
    pub ty: FType,
    pub value: BigDecimal,
}

impl TryFrom<f32> for FConst {
    type Error = ();

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        let value = BigDecimal::from_f32(value).ok_or(())?;
        Ok(Self {
            ty: FType::Fp32,
            value,
        })
    }
}

impl TryFrom<f64> for FConst {
    type Error = ();

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        let value = BigDecimal::from_f64(value).ok_or(())?;
        Ok(Self {
            ty: FType::Fp32,
            value,
        })
    }
}
