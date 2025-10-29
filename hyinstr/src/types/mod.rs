use std::collections::BTreeMap;

use crate::types::aggregate::{AnyType, AnyTypeRef, TypeRef};

pub mod aggregate;
pub mod primary;

#[derive(Debug, Clone, Default)]
pub struct TypeRegistry {
    map: BTreeMap<TypeRef, AnyType>,
    inverse_accelerator: BTreeMap<u64, TypeRef>,
}

impl TypeRegistry {
    fn ty_hash(ty: &AnyType) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        ty.hash(&mut hasher);
        hasher.finish()
    }

    fn find_type_internal(&self, ty_hash: u64, ty: &AnyType) -> Option<TypeRef> {
        if let Some(id_hint) = self.inverse_accelerator.get(&ty_hash) {
            // Potential hash collision however we should be optimistic here
            let elem = self.map.get(id_hint).unwrap();
            if elem == ty {
                return Some(*id_hint);
            }

            // Otherwise search the entire map for equality (expensive in O(n))
            // For a collection of size N, collisions happen with probability
            // E[x != y && H(x) == H(y)] = 1/(2^64) * math::comb(N, 2) = N*(N-1)/(2*2^64) ~= N^2/(2*2^64)
            //
            // For
            //   N = 100,             E[X] ~= 2.7E-16
            //   N = 10_000,          E[X] ~= 2.7E−12
            //   N = 10_000_000_000,  E[X] ~= 2.7
            //   N = 1E12,            E[X] ~= 270
            //
            // So this assumption is fairly safe for anything remotely reasonable UNDER THE ASSUMPTION THAT
            // THE HASH FUNCTION IS PERFECTLY UNIFORM (which is not true in practice).
            //
            for (existing_id, existing_ty) in &self.map {
                if existing_ty == ty {
                    return Some(*existing_id);
                }
            }

            // Guaranteed that inverse_accelerator is always referencing a valid map because insertion, does not allow deletion.
            unreachable!()
        } else {
            None
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, ty: AnyType) -> TypeRef {
        let ty_hash = Self::ty_hash(&ty);
        if let Some(existing_id) = self.find_type_internal(ty_hash, &ty) {
            return existing_id;
        }

        // Type does not seem to exist yet, insert it
        let new_id = TypeRef(self.map.len());
        self.map.insert(new_id, ty);
        self.inverse_accelerator.insert(ty_hash, new_id);
        new_id
    }

    pub fn get(&self, id: &TypeRef) -> Option<AnyTypeRef<'_>> {
        self.map.get(id).map(AnyType::as_ref)
    }

    pub fn find_type(&self, ty: &AnyType) -> Option<TypeRef> {
        let ty_hash = Self::ty_hash(ty);
        self.find_type_internal(ty_hash, ty)
    }
}
