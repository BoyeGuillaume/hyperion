//! Types module
//!
//! This module contains the canonical representation of types used by the
//! `hyinstr` crate. It exposes a small type system built on three layers:
//!
//! - Primary types: primitive and vector types (see `primary.rs`).
//! - Aggregate types: arrays and structures (see `aggregate.rs`).
//! - A registry-backed [`AnyType`] wrapper and [`TypeRegistry`] which deduplicates
//!   types and provides stable [`Typeref`] identifiers (UUID-based).
//!
//! The formatting helpers (e.g. [`AnyType::fmt`]) accept a reference to [`TypeRegistry`] so
//! that aggregate types can resolve their element types for human-friendly
//! printing.
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
/// arrays and structures. [`AnyType`] implements `Hash`/`Eq` so it can be
/// deduplicated by the [`TypeRegistry`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnyType {
    /// Primary types
    ///
    /// All types that can be represented as [`PrimaryType`]. Those are typically non-composite types. These include:
    /// - Integer types (eg., `i8`, `i32`, `i64`, etc.)
    /// - Floating-point types (eg., `f32`, `f64`)
    /// - Vector types (eg., `v4i32`, `v8f16`, etc.)
    /// - Pointer types (opaque)
    ///
    Primary(PrimaryType),

    /// An array type: element typeref + element count.
    ///
    /// Notice that the number of elements MUST be known at compile time. This is inadequate for
    /// representing dynamically sized arrays.
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
    fn internal_fmt<U>(&self, ref_object: U) -> impl std::fmt::Display
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

    /// Build a formatting helper that renders this type using the provided
    /// registry to resolve referenced element types.
    ///
    /// Example:
    /// ```rust
    /// # use hyinstr::types::{AnyType, TypeRegistry, primary::IType};
    /// let reg = TypeRegistry::new([0; 6]);
    /// let t = AnyType::from(IType::I32);
    /// assert_eq!(format!("{}", t.fmt(&reg)), "i32");
    /// ```
    pub fn fmt<'a>(&'a self, registry: &'a TypeRegistry) -> impl std::fmt::Display {
        self.internal_fmt(registry.array.read_recursive())
    }
}

/// A central registry that stores and deduplicates `AnyType` values.
///
/// The registry provides fast lookup by `Typeref` and ensures identical type
/// descriptions map to the same stable identifier.
///
///
/// Example:
///
/// ```rust
/// # use hyinstr::types::{TypeRegistry, primary::IType};
/// # use std::sync::Arc;
///
/// let reg = TypeRegistry::new([0u8; 6]);
/// let typeref = reg.search_or_insert(IType::I8.into());
/// assert_eq!(reg.search_or_insert(IType::I8.into()), typeref);
/// assert_eq!(reg.get(typeref).as_deref(), Some(&IType::I8.into()));
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

    /// Create a new [`TypeRegistry`] instance.
    ///
    /// `node_id` is used when allocating UUIDs for newly inserted types.
    pub fn new(node_id: [u8; 6]) -> Self {
        Self {
            array: Default::default(),
            inverse_lookup: Default::default(), // INFO: Always lock array before inverse_lookup to avoid deadlock
            context: uuid::timestamp::context::Context::new(0),
            node_id,
        }
    }

    /// Retrieve a borrowed [`AnyType`] for the given `typeref`. Returns
    /// [`None`] if the given `typeref` is not present in the registry.
    ///
    /// # A note on concurrency
    /// This method internally acquires a read lock on the type storage. As a
    /// result,
    ///  1) Multiple concurrent readers are allowed.
    ///  2) You mustn't hold a read-guard while calling [`Self::search_or_insert`] as
    ///     it may attempt to upgrade to a write lock, leading to a deadlock.
    ///  3) The returned guard keeps the read lock held for the lifetime of the guard.
    ///
    /// Example:
    /// ```rust
    /// # use hyinstr::types::{TypeRegistry, primary::IType};
    /// let reg = TypeRegistry::new([0; 6]);
    /// let typeref = reg.search_or_insert(IType::I32.into());
    /// let guard1 = reg.get(typeref).unwrap();
    /// let guard2 = reg.get(typeref).unwrap();
    /// assert_eq!(&*guard1, &IType::I32.into());
    /// assert_eq!(&*guard2, &IType::I32.into());
    /// ```
    pub fn get(&self, typeref: Typeref) -> Option<MappedRwLockReadGuard<'_, AnyType>> {
        let array_lock = self.array.read_recursive();

        // Acquire the typeref
        RwLockReadGuard::try_map(array_lock, |map| map.get(&typeref.0)).ok()
    }

    /// Insert `ty` into the registry if an equivalent type doesn't already
    /// exist and return the [`Typeref`] for it.
    ///
    /// If an identical type is already present, its existing [`Typeref`] is returned,
    /// otherwise if not, a new UUID is allocated and the type is inserted.
    ///
    /// # A note on concurrency
    /// This method internally acquires read locks on the type storage, and
    /// upgrades them to write locks if a new type must be inserted. As a result,
    ///  1) You **MUST NOT** hold a read-guard returned by [`Self::get`] while calling this method,
    ///     as it may attempt to upgrade to a write lock, leading to a deadlock.
    ///  2) Multiple concurrent readers are allowed, but writers are exclusive.
    ///  3) If you also hold a guard returned by [`Self::get`], release it before calling
    ///     this method.
    ///  4) The method uses an "upgradable read lock" pattern to minimize write lock
    ///     contention. We further assume that writes are rare compared to reads, motivating
    ///     this design.
    ///
    /// # About hash collisions
    /// The registry uses a hash-based inverse lookup to quickly find candidate types. This section describes
    /// the access the probability of hash collisions (which are very rare in practice) and how they are handled.
    ///
    /// - Assuming a perfectly uniform hash function, the expected number of collisions E\[X\] for N types is:
    ///
    ///   E[x != y && H(x) == H(y)] = 1 / (2^64) * math::comb(N, 2) ~= N^2/(2^65)
    ///
    /// - In practice, for:
    ///   | N      | Expected Collisions E\[X\] |
    ///   |--------|----------------------------|
    ///   | 100    | 2.7E-16                    |
    ///   | 10_000 | 2.7E-12                    |
    ///   | 1E10   | 2.7                        |
    ///   | 1E12   | 270                        |
    ///
    /// - As such collisions are either the consequence of 1) adversarial inputs or 2) bad hash functions, 3) extremely large type sets.
    ///   In practice such collisions only impact performance downgrading it from O(log N) to O(N log N) in the worst case for lookups.
    ///
    pub fn search_or_insert(&self, ty: AnyType) -> Typeref {
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

    /// Format a given `Typeref` using this registry.
    pub fn fmt(&self, typeref: Typeref) -> impl std::fmt::Display {
        struct Fmt<'a> {
            registry: &'a TypeRegistry,
            typeref: Typeref,
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.registry.get(self.typeref) {
                    Some(ty_guard) => ty_guard.fmt(self.registry).fmt(f),
                    None => write!(f, "<unknown type {}>", self.typeref.0),
                }
            }
        }

        Fmt {
            registry: self,
            typeref,
        }
    }
}
