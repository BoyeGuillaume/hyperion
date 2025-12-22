//! Floating‑point constants used as immediate operands.
use crate::types::primary::FType;
use bigdecimal::{BigDecimal, FromPrimitive};
use num_bigint::BigInt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A floating‑point literal paired with its `FType`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FConst {
    /// Floating-point type describing how to interpret `value`.
    pub ty: FType,

    /// Literal payload stored as an arbitrary-precision decimal.
    pub value: BigDecimal,
}

impl FConst {
    /// Create a new `FConst` from its type and value.
    pub fn new(ty: FType, value: BigDecimal) -> Self {
        Self { ty, value }
    }

    /// Create a rational based on a numerator and denominator and a floating-point type.
    pub fn from_ratio(
        ty: FType,
        numerator: impl Into<BigInt>,
        denominator: impl Into<BigInt>,
    ) -> Self {
        let num = BigDecimal::from(numerator.into());
        let denom = BigDecimal::from(denominator.into());
        let value = num / denom;
        Self { ty, value }
    }

    /// Convert the current instance into another floating-point type.
    pub fn to_type(self, new_ty: FType) -> Self {
        Self {
            ty: new_ty,
            value: self.value,
        }
    }
}

impl std::fmt::Display for FConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.ty, self.value)
    }
}

impl TryFrom<f32> for FConst {
    type Error = ();

    /// Convert a Rust `f32` into an `FConst` of type `Fp32`.
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

    /// Convert a Rust `f64` into an `FConst` of type `Fp32`.
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        let value = BigDecimal::from_f64(value).ok_or(())?;
        Ok(Self {
            ty: FType::Fp32,
            value,
        })
    }
}
