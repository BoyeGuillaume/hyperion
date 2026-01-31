//! Aggregate types
//!
//! This file provides composite types built from [`Typeref`] references stored
//! in the central [`TypeRegistry`]:
//! - [`ArrayType`]: a fixed-size array of elements referenced by [`Typeref`].
//! - [`StructType`]: a packed sequence of element [`Typeref`]s.
//!
//! Both types carry lightweight `fmt` helpers that accept a `&TypeRegistry`] so
//! that elements can be resolved for display purposes.
use std::{borrow::Borrow, collections::BTreeMap, fmt::Debug, ops::Deref};

use crate::types::{AnyType, TypeRegistry, Typeref};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Array type
///
/// An array type represents a fixed-size sequence of elements of a specified type. Notice
/// that this is very similar to [`super::primary::VcType`], however unlike vectors, arrays
/// can
///    1) be nested and reference other aggregate types
///    2) cannot leverage SIMD instructions for operations
///    3) must have a fixed size (ie., unlike [`super::primary::VcSize`])
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct ArrayType {
    /// Element type of the array.
    pub ty: Typeref,
    /// Number of elements in the array.
    pub num_elements: u16,
}

impl ArrayType {
    pub(super) fn internal_fmt<U>(&self, ref_object: U) -> impl std::fmt::Display
    where
        U: Deref<Target = BTreeMap<Uuid, AnyType>> + Sized,
    {
        struct ArrayTypeFmt<'a, U> {
            r#ref: &'a ArrayType,
            ref_object: U,
        }

        impl<U: Deref<Target = BTreeMap<Uuid, AnyType>> + Sized> std::fmt::Display for ArrayTypeFmt<'_, U> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let elem = self.ref_object.borrow().get(&self.r#ref.ty.0).unwrap();
                write!(
                    f,
                    "[ {} x {} ]",
                    self.r#ref.num_elements,
                    (*elem).internal_fmt(self.ref_object.deref()),
                )
            }
        }

        ArrayTypeFmt {
            r#ref: self,
            ref_object,
        }
    }

    /// Build a formatting helper for this `ArrayType`.
    pub fn fmt<'a>(&'a self, registry: &'a TypeRegistry) -> impl std::fmt::Display {
        self.internal_fmt(registry.array.read_recursive())
    }
}

/// Structure type
///
/// A structure type represents a sequence of elements of specified types. Structures can be
/// marked as `packed`, which indicates that their elements are laid out without padding
/// between them.
///
/// Note that structure types do not support named fields; elements are accessed by their
/// index within the structure.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct StructType {
    /// Element types of the structure, in order.
    pub element_types: Vec<Typeref>,
    /// Whether the structure is packed (no padding between elements).
    pub packed: bool,
}

impl StructType {
    pub(super) fn internal_fmt<U>(&self, ref_object: U) -> impl std::fmt::Display
    where
        U: Deref<Target = BTreeMap<Uuid, AnyType>> + Sized,
    {
        struct StructTypeFmt<'a, U> {
            r#ref: &'a StructType,
            ref_object: U,
        }

        impl<U: Deref<Target = BTreeMap<Uuid, AnyType>> + Sized> std::fmt::Display
            for StructTypeFmt<'_, U>
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if self.r#ref.packed {
                    write!(f, "packed ")?;
                }
                write!(f, "{{")?;

                let mut first = true;
                for typeref in self.r#ref.element_types.iter() {
                    let elem = self.ref_object.deref().get(&typeref.0).unwrap();
                    if !first {
                        write!(f, ", ")?;
                    } else {
                        first = false;
                    }
                    write!(f, "{}", elem.internal_fmt(self.ref_object.deref()))?;
                }

                write!(f, "}}")
            }
        }

        StructTypeFmt {
            r#ref: self,
            ref_object,
        }
    }

    /// Build a formatting helper for this `StructType`.
    pub fn fmt<'a>(&'a self, registry: &'a TypeRegistry) -> impl std::fmt::Display {
        self.internal_fmt(registry.array.read_recursive())
    }
}
