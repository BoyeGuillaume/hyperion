use std::{
    collections::{HashMap, HashSet, hash_set},
    sync::Arc,
};

use either::Either;
use hyinstr::modules::symbol::FunctionPointer;
use log::debug;
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    specifications::base::Specification,
    utils::ref_id::{PtrArcId, RefId},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SpecificationKey {
    referenced_functions: SmallVec<FunctionPointer, 2>,
}

/// A library of function specifications indexed for efficient retrieval.
///
/// This structure allows storing and querying specifications based on
/// the functions they reference.
#[derive(Debug, Clone, Default)]
pub struct SpecLibrary {
    specs: HashMap<SpecificationKey, Vec<Arc<Specification>>>,
    weak_index: HashMap<FunctionPointer, Vec<Arc<Specification>>>,
    uuid_index: HashMap<Uuid, Arc<Specification>>,
}

impl SpecLibrary {
    fn iter_from_hashset<'a>(
        hashset: HashSet<PtrArcId<'a, Specification>>,
    ) -> impl Iterator<Item = &'a Specification> {
        // Finally, return an iterator over the intersected specifications
        struct MyIter<'a> {
            inner: hash_set::IntoIter<PtrArcId<'a, Specification>>,
            _marker: std::marker::PhantomData<&'a ()>,
        }

        impl<'a> Iterator for MyIter<'a> {
            type Item = &'a Specification;

            fn next(&mut self) -> Option<Self::Item> {
                self.inner
                    .next()
                    .map(|arc_ref_id| arc_ref_id.take().as_ref())
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
        }

        MyIter {
            inner: hashset.into_iter(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Retrieve a list of specifications (strongly) matching the given referenced function pointers.
    ///
    /// If no specifications match, an empty iterator is returned.
    pub fn get(
        &self,
        key: impl IntoIterator<Item = FunctionPointer>,
    ) -> impl Iterator<Item = &Specification> {
        // Note: heap allocation here is unfortunate, find a better way.
        let key = SpecificationKey {
            referenced_functions: key.into_iter().collect(),
        };

        if let Some(elements) = self.specs.get(&key) {
            Either::Left(elements.iter().map(Arc::as_ref))
        } else {
            Either::Right(std::iter::empty())
        }
    }

    /// Retrieve a specification by its UUID.
    ///
    /// If no specification with the given UUID exists, `None` is returned.
    pub fn get_by_uuid(&self, uuid: Uuid) -> Option<&Specification> {
        self.uuid_index.get(&uuid).map(Arc::as_ref)
    }

    /// Insert a new specification into the library.
    pub fn insert(&mut self, mut spec: Specification) -> Uuid {
        spec.uuid = Uuid::new_v4();
        spec.derive_meta_asserts();
        spec.derive_meta_assumptions();
        spec.derive_referenced_functions();
        let mut elem: SmallVec<FunctionPointer, 2> =
            spec.list_referenced_functions().iter().cloned().collect();
        elem.sort();

        // Add new specification to the library (and indexes)
        let key = SpecificationKey {
            referenced_functions: elem,
        };
        let arc_spec = Arc::new(spec);
        self.specs.entry(key).or_default().push(arc_spec.clone());
        let uuid_index_result = self.uuid_index.insert(arc_spec.uuid, arc_spec.clone());
        debug_assert!(
            uuid_index_result.is_none(),
            "Specification UUID collision detected in SpecLibrary"
        );
        for func in arc_spec.list_referenced_functions() {
            self.weak_index
                .entry(func.clone())
                .or_default()
                .push(arc_spec.clone());
        }

        // Log insertion
        debug!(
            "Inserted specification {} into library referencing functions {:?}",
            arc_spec.uuid,
            arc_spec.list_referenced_functions()
        );
        arc_spec.uuid
    }

    /// Query specifications that reference all of the given function pointers.
    ///
    /// Query all specifications that reference every function pointer in `keys`.
    /// If no specifications match, an empty iterator is returned.
    pub fn query_intersect(
        &self,
        keys: impl IntoIterator<Item = FunctionPointer>,
    ) -> impl Iterator<Item = &Specification> {
        let mut keys = keys.into_iter();
        if let Some(first_key) = keys.next() {
            let mut intersected: HashSet<_> = self
                .weak_index
                .get(&first_key)
                .unwrap()
                .iter()
                .map(PtrArcId::new)
                .collect();

            // Intersect with the rest of the keys
            while let Some(key) = keys.next() {
                if let Some(specs) = self.weak_index.get(&key) {
                    for spec in specs {
                        let arc_ref_id = RefId::new(spec);
                        intersected.remove(&arc_ref_id);
                    }
                } else {
                    intersected.clear();
                    break;
                }
            }

            // Finally, return an iterator over the intersected specifications
            Either::Left(Self::iter_from_hashset(intersected))
        } else {
            Either::Right(std::iter::empty())
        }
    }

    /// Query specifications that reference any of the given function pointers.
    ///
    /// Query all specifications that reference at least one function pointer in `keys`.
    /// If no specifications match, an empty iterator is returned.
    pub fn query_union(
        &self,
        keys: impl IntoIterator<Item = FunctionPointer>,
    ) -> impl Iterator<Item = &Specification> {
        let mut unioned: HashSet<PtrArcId<'_, Specification>> = HashSet::new();

        for key in keys {
            if let Some(specs) = self.weak_index.get(&key) {
                for spec in specs {
                    unioned.insert(RefId::new(spec));
                }
            }
        }

        Self::iter_from_hashset(unioned)
    }
}
