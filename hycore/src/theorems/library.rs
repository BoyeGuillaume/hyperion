use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use uuid::Uuid;

use crate::theorems::base::Theorem;

/// A library for managing multiple [`Theorem`]s.
#[derive(Default)]
pub struct TheoremLibrary {
    specifications: DashMap<Uuid, Theorem>,
}

impl TheoremLibrary {
    /// Creates a new, empty [`TheoremLibrary`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new [`Theorem`] into the library.
    pub fn insert(&self, spec: Theorem) {
        self.specifications.insert(spec.uuid(), spec);
    }

    /// Retrieves a reference to a [`Theorem`] by its UUID.
    ///
    /// May deadlock if called when holding a mutable reference to the library.
    pub fn get(&self, uuid: &Uuid) -> Option<Ref<'_, Uuid, Theorem>> {
        self.specifications.get(uuid)
    }

    /// Retrieves a mutable reference to a [`Theorem`] by its UUID.
    ///
    /// May deadlock if called when holding an immutable reference to the library.
    pub fn get_mut(&mut self, uuid: &Uuid) -> Option<RefMut<'_, Uuid, Theorem>> {
        self.specifications.get_mut(uuid)
    }

    /// Removes a [`Theorem`] from the library by its UUID.
    ///
    /// May deadlock if called when holding an immutable reference to the library.
    pub fn remove(&mut self, uuid: &Uuid) -> Option<Theorem> {
        self.specifications.remove(uuid).map(|x| x.1)
    }

    /// Returns an iterator over all [`Theorem`]s in the library.
    ///
    /// May deadlock if called when holding any reference to the library.
    ///
    pub fn iter(&self) -> dashmap::iter::Iter<'_, Uuid, Theorem> {
        self.specifications.iter()
    }
}
