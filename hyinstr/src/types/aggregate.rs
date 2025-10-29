#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

use crate::types::primary::PrimaryBaseType;

/// An identifier type used to reference types within a context.
///
/// I would prefer to name this `TypeId` but that conflicts with the std library.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TypeRef(pub(super) usize);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArrayType {
    pub num_elements: usize,
    pub element_type: TypeRef,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StructType {
    pub element_types: Vec<TypeRef>,
}

#[derive(Debug, Clone, EnumIs, EnumTryAs, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnyType {
    Array(ArrayType),
    Struct(StructType),
    Primary(PrimaryBaseType),
}

impl AnyType {
    pub fn as_ref<'a>(&'a self) -> AnyTypeRef<'a> {
        AnyTypeRef::from(self)
    }
}

impl From<ArrayType> for AnyType {
    fn from(at: ArrayType) -> Self {
        AnyType::Array(at)
    }
}

impl From<StructType> for AnyType {
    fn from(st: StructType) -> Self {
        AnyType::Struct(st)
    }
}

impl<T: Into<PrimaryBaseType>> From<T> for AnyType {
    fn from(pt: T) -> Self {
        AnyType::Primary(pt.into())
    }
}

pub enum AnyTypeRef<'a> {
    Array(&'a ArrayType),
    Struct(&'a StructType),
    Primary(&'a PrimaryBaseType),
}

impl<'a> From<&'a AnyType> for AnyTypeRef<'a> {
    fn from(any_type: &'a AnyType) -> Self {
        match any_type {
            AnyType::Array(arr) => AnyTypeRef::Array(arr),
            AnyType::Struct(st) => AnyTypeRef::Struct(st),
            AnyType::Primary(pt) => AnyTypeRef::Primary(pt),
        }
    }
}
