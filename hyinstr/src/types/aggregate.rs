use crate::types::{TypeRegistry, Typeref};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Array type
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArrayType {
    pub ty: Typeref,
    pub num_elements: usize,
}

impl ArrayType {
    pub fn fmt<'a>(&'a self, registry: &'a TypeRegistry) -> ArrayTypeFmt<'a> {
        ArrayTypeFmt {
            r#ref: self,
            registry,
        }
    }
}

pub struct ArrayTypeFmt<'a> {
    r#ref: &'a ArrayType,
    registry: &'a TypeRegistry,
}

impl std::fmt::Display for ArrayTypeFmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let elem = self.registry.get(self.r#ref.ty).unwrap();
        write!(
            f,
            "[ {} x {} ]",
            self.r#ref.num_elements,
            (*elem).fmt(self.registry)
        )
    }
}

/// Structure type
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StructType {
    pub element_types: Vec<Typeref>,
}

impl StructType {
    pub fn fmt<'a>(&'a self, registry: &'a TypeRegistry) -> StructTypeFmt<'a> {
        StructTypeFmt { ty: self, registry }
    }
}

pub struct StructTypeFmt<'a> {
    ty: &'a StructType,
    registry: &'a TypeRegistry,
}

impl std::fmt::Display for StructTypeFmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "type {{")?;

        for typeref in self.ty.element_types.iter() {
            let elem = self.registry.get(*typeref).unwrap();
            write!(f, "{}", elem.fmt(self.registry))?;
        }

        write!(f, "}}")
    }
}
