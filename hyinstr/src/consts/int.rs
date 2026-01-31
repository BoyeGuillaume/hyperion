//! Integer constants used as immediate operands.
use num_bigint::BigInt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::primary::IType;

/// An integer literal paired with its `IType` width.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IConst {
    /// Integer type describing the bit-width of the literal.
    pub ty: IType,
    /// Literal payload stored with unbounded precision.
    pub value: BigInt,
}

/// Serialize a [`BigInt`] using Borsh
#[cfg(feature = "borsh")]
pub fn serialize_bigint_borsh<W: std::io::Write>(
    value: &BigInt,
    writer: &mut W,
) -> std::io::Result<()> {
    let bytes = value.to_signed_bytes_le();
    borsh::BorshSerialize::serialize(&bytes, writer)
}

/// Deserialize a [`BigInt`] using Borsh
#[cfg(feature = "borsh")]
pub fn deserialize_bigint_borsh<R: std::io::Read>(reader: &mut R) -> std::io::Result<BigInt> {
    let bytes: Vec<u8> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    Ok(BigInt::from_signed_bytes_le(&bytes))
}

#[cfg(feature = "borsh")]
impl borsh::BorshSerialize for IConst {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        borsh::BorshSerialize::serialize(&self.ty, writer)?;
        serialize_bigint_borsh(&self.value, writer)?;
        Ok(())
    }
}

#[cfg(feature = "borsh")]
impl borsh::BorshDeserialize for IConst {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let ty = borsh::BorshDeserialize::deserialize_reader(reader)?;
        let value = deserialize_bigint_borsh(reader)?;
        Ok(Self { ty, value })
    }
}

impl std::fmt::Display for IConst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.ty, self.value)
    }
}

impl From<u8> for IConst {
    /// Create an 8‑bit integer constant from a primitive value.
    fn from(value: u8) -> Self {
        Self {
            ty: IType::I8,
            value: value.into(),
        }
    }
}

impl From<u16> for IConst {
    /// Create a 16‑bit integer constant from a primitive value.
    fn from(value: u16) -> Self {
        Self {
            ty: IType::I16,
            value: value.into(),
        }
    }
}

impl From<u32> for IConst {
    /// Create a 32‑bit integer constant from a primitive value.
    fn from(value: u32) -> Self {
        Self {
            ty: IType::I32,
            value: value.into(),
        }
    }
}

impl From<u64> for IConst {
    /// Create a 64‑bit integer constant from a primitive value.
    fn from(value: u64) -> Self {
        Self {
            ty: IType::I64,
            value: value.into(),
        }
    }
}
