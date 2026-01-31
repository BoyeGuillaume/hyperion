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

use auto_enums::auto_enum;
use log::{debug, info};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use strum::{EnumIs, EnumTryAs};
use uuid::{Timestamp, Uuid};

use crate::types::{
    aggregate::{ArrayType, StructType},
    primary::{PrimaryType, WType},
};
pub mod aggregate;
pub mod checker;
pub mod primary;

/// A stable reference to a type stored inside a `TypeRegistry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Typeref(Uuid);

impl Typeref {
    fn internal_is_wildcard(uuid: &Uuid) -> bool {
        // A wildcard type is identified as a custom version 8 UUID
        if uuid.get_version() == Some(uuid::Version::Custom) {
            // If fed with the following bytes, XX marks overridden bits:
            // 0xf, 0xe, 0xd, 0xc, 0xb, 0xa, XX, 0x8, XX, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1, 0x0,
            // By convention, we put the wildcard ID in the last two bytes (bytes 14 and 15) and everything else as 0xff
            let bytes = uuid.as_bytes();
            if bytes[0..6] == [0xff; 6] && bytes[7] == 0xff && bytes[9..13] == [0xff; 4] {
                return true;
            }
        }

        false
    }

    /// Check whether the current `Typeref` is a wildcard type.
    pub fn is_wildcard(&self) -> bool {
        Self::internal_is_wildcard(&self.0)
    }

    /// Create a new wildcard `Typeref` with the given `id`.
    pub fn new_wildcard(id: u16) -> Self {
        let bytes = id.to_le_bytes();

        // Pad the remaining bytes with zeros
        let mut buf = [0xffu8; 16];
        buf[14..16].copy_from_slice(&bytes);
        Self(Uuid::new_v8(buf))
    }

    /// Retrieve the wildcard ID if this is a wildcard type.
    pub(crate) fn wildcard_id(&self) -> Option<u16> {
        if self.is_wildcard() {
            let bytes = self.0.as_bytes();
            Some(u16::from_le_bytes([bytes[14], bytes[15]]))
        } else {
            None
        }
    }

    /// Try to retrieve the wildcard from the current `Typeref`. Returns `None` if
    /// this is not a wildcard type.
    pub fn try_as_wildcard(&self) -> Option<WType> {
        self.wildcard_id().map(|id| WType { id })
    }

    /// Retrieve the wildcard from the current `Typeref`. Panics if this is not
    /// a wildcard type.
    pub fn as_wildcard(&self) -> WType {
        self.try_as_wildcard()
            .expect("Typeref is not a wildcard type")
    }
}

/// A sum-type representing any type that can be stored in the registry.
///
/// This includes primary (primitive/vector) types, aggregate types like
/// arrays and structures. [`AnyType`] implements `Hash`/`Eq` so it can be
/// deduplicated by the [`TypeRegistry`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIs, EnumTryAs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
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

    /// Iterate over all [`Typeref`]s referenced by this type.
    #[auto_enum(Iterator)]
    pub fn iter_referenced_typerefs(&self) -> impl Iterator<Item = Typeref> {
        match self {
            AnyType::Primary(_) => std::iter::empty(),
            AnyType::Array(array_type) => std::iter::once(array_type.ty),
            AnyType::Struct(struct_type) => struct_type.element_types.iter().cloned(),
        }
    }

    /// Mutably iterate over all [`Typeref`]s referenced by this type.
    #[auto_enum(Iterator)]
    pub fn iter_referenced_typerefs_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        match self {
            AnyType::Primary(_) => std::iter::empty(),
            AnyType::Array(array_type) => std::iter::once(&mut array_type.ty),
            AnyType::Struct(struct_type) => struct_type.element_types.iter_mut(),
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
    inverse_lookup: RwLock<BTreeMap<u64, SmallVec<Uuid, 1>>>,
    context: uuid::timestamp::context::Context,
    node_id: [u8; 6],
}

#[cfg(feature = "borsh")]
impl borsh::BorshSerialize for TypeRegistry {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let array_lock = self.array.read();

        // Write number of types
        let len = array_lock.len() as u64;
        <u64 as borsh::BorshSerialize>::serialize(&len, writer)?;

        // Write each type
        for (uuid, ty) in array_lock.iter() {
            <Uuid as borsh::BorshSerialize>::serialize(uuid, writer)?;
            <AnyType as borsh::BorshSerialize>::serialize(ty, writer)?;
        }

        Ok(())
    }
}

#[cfg(feature = "borsh")]
impl borsh::BorshDeserialize for TypeRegistry {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // Read number of types
        let len = <u64 as borsh::BorshDeserialize>::deserialize_reader(reader)? as usize;

        let mut array = BTreeMap::new();
        let mut inverse_lookup: BTreeMap<u64, SmallVec<Uuid, 1>> = BTreeMap::new();

        for _ in 0..len {
            let uuid = <Uuid as borsh::BorshDeserialize>::deserialize_reader(reader)?;
            let ty = <AnyType as borsh::BorshDeserialize>::deserialize_reader(reader)?;

            // Insert into array
            array.insert(uuid, ty.clone());

            // Insert into inverse lookup
            let h = {
                let mut hasher = DefaultHasher::new();
                ty.hash(&mut hasher);
                hasher.finish()
            };
            if let Some(list) = inverse_lookup.get_mut(&h) {
                list.push(uuid);
            } else {
                inverse_lookup.insert(h, smallvec![uuid]);
            }
        }

        Ok(Self {
            array: RwLock::new(array),
            inverse_lookup: RwLock::new(inverse_lookup),
            context: uuid::timestamp::context::Context::new_random(),
            node_id: [0u8; 6], // NOTE: Node ID is not serialized/deserialized, a later call to init_node_id must be done
        })
    }
}

impl TypeRegistry {
    fn hash_ty(ty: &AnyType) -> u64 {
        let mut hasher = DefaultHasher::new();
        ty.hash(&mut hasher);
        hasher.finish()
    }

    fn next_uuid(&self) -> Uuid {
        let ts = Timestamp::now(&self.context);
        let uuid = Uuid::new_v6(ts, &self.node_id);
        debug_assert!(!Typeref::internal_is_wildcard(&uuid));
        uuid
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

    /// Merge this registry with another, inserting all types from `other`
    /// into `self`. Returns a mapping from `other`'s `Typeref`s to `self`'s
    /// `Typeref`s.
    pub fn merge_with(&self, other: &TypeRegistry) -> BTreeMap<Typeref, Typeref> {
        let mut mapping = BTreeMap::new();

        let other_array_lock = other.array.read_recursive();
        let mut list_of_types: Vec<(Uuid, AnyType)> = other_array_lock
            .iter()
            .map(|(uuid, ty)| (*uuid, ty.clone()))
            .collect();

        // When inserting types, we need to ensure all previous types are inserted first
        // to handle dependencies correctly. To achieve this, we first collect all types,
        // then insert all we can until none are left to insert.
        while !list_of_types.is_empty() {
            list_of_types.retain_mut(|(uuid, any_type)| {
                let can_insert = any_type.iter_referenced_typerefs().all(|ref_typeref| {
                    mapping.contains_key(&ref_typeref) || ref_typeref.is_wildcard()
                });

                // Map old typeref to new typeref
                for tyref in any_type.iter_referenced_typerefs_mut() {
                    if let Some(new_typeref) = mapping.get(tyref) {
                        *tyref = *new_typeref;
                    }
                }

                if can_insert {
                    let self_typeref = self.search_or_insert(any_type.clone());
                    mapping.insert(Typeref(*uuid), self_typeref);
                    false
                } else {
                    true
                }
            });
        }

        mapping
    }

    /// Retrieve a borrowed [`AnyType`] for the given `typeref`. Returns
    /// [`None`] if the given `typeref` is not present in the registry.
    ///
    /// Notice that [`Typeref::is_wildcard`] types are not stored in the registry, and
    /// should not be queried using this method. You can directly check for wildcard with
    /// [`Typeref::is_wildcard`] without accessing the registry. This method will panic if
    /// given a wildcard `Typeref`.
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
        if typeref.is_wildcard() {
            unreachable!()
        }

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
        if let AnyType::Primary(PrimaryType::Wildcard(wtype)) = ty {
            return Typeref::new_wildcard(wtype.id);
        }

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

    /// Number of types stored in the registry. Should be used for debugging
    /// because of concurrency.
    pub fn len(&self) -> usize {
        self.array.read().len()
    }

    /// Check whether the registry is empty. Should be used for debugging
    /// because of concurrency.
    pub fn is_empty(&self) -> bool {
        self.array.read().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::primary::IType;

    use super::*;

    #[test]
    fn test_wildcard_typeref() {
        for wildcard_id in [0u16, 1, 42, 65535, 12345, 0xFFFF, 0xABCD] {
            let typeref = Typeref::new_wildcard(wildcard_id);
            assert!(typeref.is_wildcard());
            assert_eq!(typeref.wildcard_id(), Some(wildcard_id));
            assert!(typeref.0.get_version() == Some(uuid::Version::Custom));
        }
    }

    #[test]
    fn test_non_wildcard_typeref() {
        let reg = TypeRegistry::new([0u8; 6]);
        let typeref = reg.search_or_insert(IType::I32.into());
        assert!(!typeref.is_wildcard());
        assert_eq!(typeref.wildcard_id(), None);

        let another_typeref = reg.search_or_insert(IType::I64.into());
        assert!(!another_typeref.is_wildcard());
        assert_eq!(another_typeref.wildcard_id(), None);

        let nil_test = Typeref(Uuid::nil());
        assert!(!nil_test.is_wildcard());
        assert_eq!(nil_test.wildcard_id(), None);
    }

    #[test]
    fn test_registry_insert_and_get() {
        let reg = TypeRegistry::new([0u8; 6]);
        let typeref_i32 = reg.search_or_insert(IType::I32.into());
        let typeref_i64 = reg.search_or_insert(IType::I64.into());

        assert_eq!(reg.get(typeref_i32).as_deref(), Some(&IType::I32.into()));
        assert_eq!(reg.get(typeref_i64).as_deref(), Some(&IType::I64.into()));

        // Inserting again should return the same typeref
        let typeref_i32_again = reg.search_or_insert(IType::I32.into());
        assert_eq!(typeref_i32, typeref_i32_again);
    }

    #[test]
    fn test_registry_wildcard_handling() {
        let reg = TypeRegistry::new([0u8; 6]);
        let wildcard_typeref = Typeref::new_wildcard(42);

        // Inserting a wildcard type should return the same typeref
        let returned_typeref = reg.search_or_insert(AnyType::from(PrimaryType::Wildcard(
            crate::types::primary::WType { id: 42 },
        )));
        assert_eq!(wildcard_typeref, returned_typeref);
    }

    #[test]
    #[should_panic]
    fn test_registry_get_wildcard_panics() {
        let reg = TypeRegistry::new([0u8; 6]);
        let wildcard_typeref = Typeref::new_wildcard(42);
        let _ = reg.get(wildcard_typeref);
    }

    #[test]
    fn test_registry_on_complex_types() {
        let reg = TypeRegistry::new([0u8; 6]);

        // Test registry on type {i32, <4 x i8>}
        let simd_vector_typeref = reg.search_or_insert(
            crate::types::primary::VcType {
                ty: IType::I8.into(),
                size: 4.into(),
            }
            .into(),
        );

        let i32_typeref = reg.search_or_insert(IType::I32.into());

        let struct_typeref = reg.search_or_insert(
            StructType {
                element_types: vec![i32_typeref, simd_vector_typeref],
                packed: false,
            }
            .into(),
        );

        assert_ne!(struct_typeref, i32_typeref);
        assert_ne!(struct_typeref, simd_vector_typeref);
        assert_eq!(
            reg.get(struct_typeref).as_deref(),
            Some(
                &StructType {
                    element_types: vec![i32_typeref, simd_vector_typeref],
                    packed: false,
                }
                .into()
            )
        );
        assert_eq!(reg.get(i32_typeref).as_deref(), Some(&IType::I32.into()));
        assert_eq!(
            reg.get(simd_vector_typeref).as_deref(),
            Some(
                &crate::types::primary::VcType {
                    ty: IType::I8.into(),
                    size: 4.into(),
                }
                .into()
            )
        );
    }
}
