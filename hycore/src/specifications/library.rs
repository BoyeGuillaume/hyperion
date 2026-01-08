use std::collections::BTreeMap;

use uuid::Uuid;

use crate::specifications::base::Specification;

#[derive(Default)]
pub struct SpecificationLibrary {
    specifications: BTreeMap<Uuid, Specification>,
}

impl SpecificationLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, spec: Specification) {
        self.specifications.insert(spec.uuid(), spec);
    }

    pub fn get(&self, uuid: &Uuid) -> Option<&Specification> {
        self.specifications.get(uuid)
    }

    pub fn get_mut(&mut self, uuid: &Uuid) -> Option<&mut Specification> {
        self.specifications.get_mut(uuid)
    }

    pub fn remove(&mut self, uuid: &Uuid) -> Option<Specification> {
        self.specifications.remove(uuid)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Specification> {
        self.specifications.values()
    }
}
