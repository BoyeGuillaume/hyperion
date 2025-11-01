//! Aggregate types
//!
//! This file provides composite types built from `Typeref` references stored
//! in the central `TypeRegistry`:
//! - `ArrayType`: a fixed-size array of elements referenced by `Typeref`.
//! - `StructType`: a packed sequence of element `Typeref`s.
//!
//! Both types carry lightweight `fmt` helpers that accept a `&TypeRegistry` so
//! that elements can be resolved for display purposes.
use std::{borrow::Borrow, collections::BTreeMap, fmt::Debug, ops::Deref};

use crate::types::{AnyType, TypeRegistry, Typeref};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Array type
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArrayType {
    pub ty: Typeref,
    pub num_elements: usize,
}

impl ArrayType {
    pub(super) fn internal_fmt<'a, U>(&'a self, ref_object: U) -> impl std::fmt::Display
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
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StructType {
    pub element_types: Vec<Typeref>,
}

impl StructType {
    pub(super) fn internal_fmt<'a, U>(&'a self, ref_object: U) -> impl std::fmt::Display
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
                write!(f, "type {{")?;

                for typeref in self.r#ref.element_types.iter() {
                    let elem = self.ref_object.deref().get(&typeref.0).unwrap();
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
