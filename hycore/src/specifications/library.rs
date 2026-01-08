use std::collections::BTreeMap;

use uuid::Uuid;

use crate::specifications::base::Specification;

/// A library for managing multiple [`Specification`]s.
#[derive(Default)]
pub struct SpecificationLibrary {
    specifications: BTreeMap<Uuid, Specification>,
}

impl SpecificationLibrary {
    /// Creates a new, empty [`SpecificationLibrary`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new [`Specification`] into the library.
    pub fn insert(&mut self, spec: Specification) {
        self.specifications.insert(spec.uuid(), spec);
    }

    /// Retrieves a reference to a [`Specification`] by its UUID.
    pub fn get(&self, uuid: &Uuid) -> Option<&Specification> {
        self.specifications.get(uuid)
    }

    /// Retrieves a mutable reference to a [`Specification`] by its UUID.
    pub fn get_mut(&mut self, uuid: &Uuid) -> Option<&mut Specification> {
        self.specifications.get_mut(uuid)
    }

    /// Removes a [`Specification`] from the library by its UUID.
    pub fn remove(&mut self, uuid: &Uuid) -> Option<Specification> {
        self.specifications.remove(uuid)
    }

    /// Returns an iterator over all [`Specification`]s in the library.
    pub fn iter(&self) -> impl Iterator<Item = &Specification> {
        self.specifications.values()
    }
}
