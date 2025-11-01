use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Typeref(Uuid);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnyType {
    Primary(PrimaryType),
    Array(ArrayType),
    Structure(StructType),
}

impl AnyType {
    pub fn fmt<'a>(&'a self, registry: &'a TypeRegistry) -> AnyTypeFmt<'a> {
        AnyTypeFmt { ty: self, registry }
    }
}

pub struct AnyTypeFmt<'a> {
    ty: &'a AnyType,
    registry: &'a TypeRegistry,
}

impl std::fmt::Display for AnyTypeFmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ty {
            AnyType::Primary(primary_type) => primary_type.fmt(f),
            AnyType::Array(array_type) => array_type.fmt(self.registry).fmt(f),
            AnyType::Structure(struct_type) => struct_type.fmt(self.registry).fmt(f),
        }
    }
}

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

    pub fn new(node_id: [u8; 6]) -> Self {
        Self {
            array: Default::default(),
            inverse_lookup: Default::default(), // INFO: Always lock array before inverse_lookup to avoid deadlock
            context: uuid::timestamp::context::Context::new(0),
            node_id,
        }
    }

    pub fn get(&self, typeref: Typeref) -> Option<MappedRwLockReadGuard<'_, AnyType>> {
        // Lock ro on array
        let array_lock = self.array.read_recursive();

        // Acquire the typeref
        RwLockReadGuard::try_map(array_lock, |map| map.get(&typeref.0)).ok()
    }

    pub fn get_or_insert(&self, ty: AnyType) -> Typeref {
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
                if let Some(hash) = inverse_lookup_lock.get_mut(&h) {
                    // TODO: Improve message for hash collision
                    info!("Hash collision detected on hash {}", h);
                    hash.push(new_typeref);
                } else {
                    // TODO: Improve message for hash collision
                    debug!("Inserting type translation from {} to {}", new_typeref, h);
                    inverse_lookup_lock.insert(h, smallvec![new_typeref]);
                }

                // Insert in array
                array_lock.insert(new_typeref, ty);
                Typeref(new_typeref)
            })
        })
    }
}
