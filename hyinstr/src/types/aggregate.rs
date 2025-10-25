#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

use crate::types::primary::PrimaryBaseType;

/// An identifier type used to reference types within a context.
///
/// I would prefer to name this `TypeId` but that conflicts with the std library.
///
pub type IdType = usize;

pub struct Context {}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArrayType {
    pub num_elements: usize,
    pub element_type: IdType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StructType {
    pub element_types: Vec<IdType>,
}

#[derive(Debug, Clone, EnumIs, EnumTryAs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnyType {
    Array(ArrayType),
    Struct(StructType),
    Primary(PrimaryBaseType),
}
