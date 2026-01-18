use std::collections::BTreeMap;

use uuid::Uuid;

use crate::theorems::base::Theorem;

/// A library for managing multiple [`Theorem`]s.
#[derive(Default)]
pub struct TheoremLibrary {
    specifications: BTreeMap<Uuid, Theorem>,
}

impl TheoremLibrary {
    /// Creates a new, empty [`TheoremLibrary`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new [`Theorem`] into the library.
    pub fn insert(&mut self, spec: Theorem) {
        self.specifications.insert(spec.uuid(), spec);
    }

    /// Retrieves a reference to a [`Theorem`] by its UUID.
    pub fn get(&self, uuid: &Uuid) -> Option<&Theorem> {
        self.specifications.get(uuid)
    }

    /// Retrieves a mutable reference to a [`Theorem`] by its UUID.
    pub fn get_mut(&mut self, uuid: &Uuid) -> Option<&mut Theorem> {
        self.specifications.get_mut(uuid)
    }

    /// Removes a [`Theorem`] from the library by its UUID.
    pub fn remove(&mut self, uuid: &Uuid) -> Option<Theorem> {
        self.specifications.remove(uuid)
    }

    /// Returns an iterator over all [`Theorem`]s in the library.
    pub fn iter(&self) -> impl Iterator<Item = &Theorem> {
        self.specifications.values()
    }
}
