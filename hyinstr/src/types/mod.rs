//! Types module
//!
//! This module contains the canonical representation of types used by the
//! `hyinstr` crate. It exposes a small type system built on three layers:
//!
//! - Primary types: primitive and vector types (see `primary.rs`).
//! - Aggregate types: arrays and structures (see `aggregate.rs`).
//! - A registry-backed `AnyType` wrapper and `TypeRegistry` which deduplicates
//!   types and provides stable `Typeref` identifiers (UUID-based).
//!
//! The registry is thread-safe and optimised for concurrent reads. Types are
//! hashed for quick lookup and a UUID is allocated for each distinct type.
//!
//! The formatting helpers (e.g. `AnyType::fmt`) accept a `&TypeRegistry` so
//! that aggregate types can resolve their element types for human-friendly
//! printing.
//!
//! See `README.md` in this directory for a higher-level overview and examples.
use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Deref,
};

use log::{debug, info};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use uuid::{Timestamp, Uuid};

use crate::types::{
    aggregate::{ArrayType, StructType},
    primary::PrimaryType,
};
pub mod aggregate;
pub mod primary;

/// A stable reference to a type stored inside a `TypeRegistry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Typeref(Uuid);

/// A sum-type representing any type that can be stored in the registry.
///
/// This includes primary (primitive/vector) types, aggregate types like
/// arrays and structures. `AnyType` implements `Hash`/`Eq` so it can be
/// deduplicated by the `TypeRegistry`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnyType {
    /// Primitive and vector types.
    Primary(PrimaryType),
    /// An array type: element typeref + element count.
    Array(ArrayType),
    /// A structure type: an ordered list of element typerefs.
    Struct(StructType),
}

impl<S: Into<PrimaryType>> From<S> for AnyType {
    fn from(value: S) -> Self {
        AnyType::Primary(value.into())
    }
}

impl From<ArrayType> for AnyType {
    fn from(value: ArrayType) -> Self {
        AnyType::Array(value)
    }
}

impl From<StructType> for AnyType {
    fn from(value: StructType) -> Self {
        AnyType::Struct(value)
    }
}

impl AnyType {
    fn internal_fmt<'a, U>(&'a self, ref_object: U) -> impl std::fmt::Display
    where
        U: Deref<Target = BTreeMap<Uuid, AnyType>> + Sized,
    {
        struct AnyTypeFmt<'a, U> {
            ty: &'a AnyType,
            ref_object: U,
        }

        impl<U: Deref<Target = BTreeMap<Uuid, AnyType>> + Sized> std::fmt::Display for AnyTypeFmt<'_, U> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.ty {
                    AnyType::Primary(primary_type) => primary_type.fmt(f),
                    AnyType::Array(array_type) => {
                        array_type.internal_fmt(self.ref_object.deref()).fmt(f)
                    }
                    AnyType::Struct(struct_type) => {
                        struct_type.internal_fmt(self.ref_object.deref()).fmt(f)
                    }
                }
            }
        }

        AnyTypeFmt {
            ty: self,
            ref_object,
        }
    }

    pub fn fmt<'a>(&'a self, registry: &'a TypeRegistry) -> impl std::fmt::Display {
        self.internal_fmt(registry.array.read_recursive())
    }
}

/// A central registry that stores and deduplicates `AnyType` values.
///
/// The registry is thread-safe: it uses `parking_lot::RwLock` for concurrent
/// access and follows a strict locking order to avoid deadlocks.
///
///
/// Example:
///
/// ```rust
/// use hyinstr::types::{TypeRegistry, primary::IType};
/// use std::sync::Arc;
///
/// fn main() {
///   let reg = TypeRegistry::new([0u8; 6]);
///   let typeref = reg.search_or_insert(IType::I8.into());
///   assert_eq!(reg.search_or_insert(IType::I8.into()), typeref);
///   assert_eq!(reg.get(typeref).as_deref(), Some(&IType::I8.into()));
/// }
/// ```
pub struct TypeRegistry {
    array: RwLock<BTreeMap<Uuid, AnyType>>,
    inverse_lookup: RwLock<BTreeMap<u64, SmallVec<[Uuid; 1]>>>,
    context: uuid::timestamp::context::Context,
    node_id: [u8; 6],
}

impl TypeRegistry {
    fn hash_ty(ty: &AnyType) -> u64 {
        let mut hasher = DefaultHasher::new();
        ty.hash(&mut hasher);
        hasher.finish()
    }

    fn next_uuid(&self) -> Uuid {
        let ts = Timestamp::now(&self.context);
        Uuid::new_v6(ts, &self.node_id)
    }

    /// Create a new `TypeRegistry`.
    ///
    /// `node_id` is used when allocating UUIDs for newly inserted types. The
    /// registry starts empty and uses internal `RwLock`s to permit many
    /// concurrent readers while allowing safe upgrades to write locks when a
    /// new type must be inserted.
    pub fn new(node_id: [u8; 6]) -> Self {
        Self {
            array: Default::default(),
            inverse_lookup: Default::default(), // INFO: Always lock array before inverse_lookup to avoid deadlock
            context: uuid::timestamp::context::Context::new(0),
            node_id,
        }
    }

    /// Retrieve a borrowed `AnyType` for the given `Typeref`.
    ///
    /// Returns `None` if the `Typeref` is not present in the registry. The
    /// returned guard implements `Deref<Target=AnyType>` and keeps the read
    /// lock held for the lifetime of the guard. Prefer this method over
    /// `get_or_insert` when you only need to read an existing type.
    pub fn get(&self, typeref: Typeref) -> Option<MappedRwLockReadGuard<'_, AnyType>> {
        let array_lock = self.array.read_recursive();

        // Acquire the typeref
        RwLockReadGuard::try_map(array_lock, |map| map.get(&typeref.0)).ok()
    }

    /// Insert `ty` into the registry if an equivalent type doesn't already
    /// exist and return the `Typeref` for it.
    ///
    /// This method first computes a 64-bit hash of the type and consults an
    /// inverse lookup map to find candidates. Any matching `AnyType` is
    /// compared for equality; if found, its existing `Typeref` is returned.
    /// Otherwise a new UUID is allocated and the type is inserted. The
    /// implementation uses upgradable read locks and upgrades them to writes
    /// only when necessary to avoid blocking readers.
    ///
    /// WARNING: Beware of dead-locks this method cannot be used from thread
    /// holding references to types (outputted by `TypeRegistry::get` method)
    pub fn search_or_insert(&self, ty: AnyType) -> Typeref {
        // Collision are very are assuming hash function is perfectly uniform
        // For a collection of size N, we have in expectation
        //
        // E[x != y && H(x) == H(y)] =
        //         1 / (2^64) * math::comb(N, 2) ~= N^2/(2^65)
        //
        // For
        //   N = 100,             E[X] ~= 2.7E-16
        //   N = 10_000,          E[X] ~= 2.7E−12
        //   N = 10_000_000_000,  E[X] ~= 2.7
        //   N = 1E12,            E[X] ~= 270/
        let h = Self::hash_ty(&ty);

        // Lock, notice that the order is critical, always lock first database first
        let mut array_lock = self.array.upgradable_read();
        let mut inverse_lookup_lock = self.inverse_lookup.upgradable_read();

        // Check if it exists in the inverse_lookup
        let typerefs = inverse_lookup_lock.get(&h);
        if let Some(typerefs) = typerefs {
            for typeref in typerefs {
                // Verify if matching
                let elem = &array_lock[typeref];
                if elem == &ty {
                    return Typeref(*typeref);
                }
            }
        }

        // Otherwise if no matches, we inverse the next type
        // NOTE: Ordering of upgrade is paramount to avoid deadlock
        array_lock.with_upgraded(|array_lock| {
            inverse_lookup_lock.with_upgraded(|inverse_lookup_lock| {
                // Reserve a new typeref
                let new_typeref = self.next_uuid();

                // Insert in the inverse_lookup_lock
                if let Some(list) = inverse_lookup_lock.get_mut(&h) {
                    // Important: log collisions at info level with full context.
                    info!("Detected an hash collision on hash 0x{:016x}. The following types collided:\n{}",
                        h,
                        list.iter().map(|uuid| {
                            format!(" - {} -> {}", uuid, array_lock.get(uuid).unwrap().internal_fmt(&*array_lock))
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    );

                    // Extra debug detail for the inverse lookup structure.
                    debug!("Inverse lookup updated for hash 0x{:016x}: {:?} (type {})", h, list, ty.internal_fmt(&*array_lock));
                    list.push(new_typeref);
                } else {
                    // Normal insertion is a debug-level event.
                    debug!("New type encountered {}. Registered with UUID {}.", ty.internal_fmt(&*array_lock), new_typeref);
                    inverse_lookup_lock.insert(h, smallvec![new_typeref]);
                }

                // Insert in array
                array_lock.insert(new_typeref, ty);
                Typeref(new_typeref)
            })
        })
    }
}
