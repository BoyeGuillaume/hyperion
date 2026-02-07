//! Constant values
//!
//! Literal values used as immediate operands in instructions. Both integer
//! and floating‑point constants are supported, with arbitrary precision types
//! where appropriate.
use crate::{
    consts::{fp::FConst, int::IConst},
    modules::{Module, Symbol, symbol::Pointer},
    types::{
        TypeRegistry, Typeref,
        aggregate::{ArrayType, StructType},
        primary::PtrType,
    },
    utils::Error,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

pub mod fp;
pub mod int;

/// A constant value (integer or floating‑point) usable as an immediate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIs, EnumTryAs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum AnyConst {
    /// Integer constant
    Int(IConst),

    /// Floating‑point constant
    Float(FConst),

    /// An array of constants (must be uniformed type)
    Array { elements: Vec<AnyConst> },

    /// A structure of constants (can be heterogeneous type)
    Struct {
        elements: Vec<AnyConst>,
        packed: bool,
    },

    /// Pointer to either a function or a global variable
    Ptr(Pointer),
}

impl AnyConst {
    pub fn verify(&self, type_registry: &TypeRegistry) -> Result<(), Error> {
        match self {
            AnyConst::Array { elements } => {
                if !elements.is_empty() {
                    let first_type = elements[0].typeref(type_registry);
                    for elem in elements.iter().skip(1) {
                        let ty = elem.typeref(type_registry);
                        if ty != first_type {
                            return Err(Error::IllegalState(format!(
                                "Array constant elements must be of uniform type. Expected type {}, found type {}.",
                                type_registry.fmt(first_type),
                                type_registry.fmt(ty)
                            )));
                        }
                    }
                }
                Ok(())
            }
            AnyConst::Struct { elements, .. } => {
                // Recursively verify all elements of the struct
                for elem in elements {
                    elem.verify(type_registry)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Retrieve the type of the constant.
    pub fn typeref(&self, type_registry: &TypeRegistry) -> Typeref {
        match self {
            AnyConst::Int(ic) => type_registry.search_or_insert(ic.ty.into()),
            AnyConst::Float(fc) => type_registry.search_or_insert(fc.ty.into()),
            AnyConst::Ptr(_) => type_registry.search_or_insert(PtrType.into()),
            AnyConst::Array { elements } => {
                debug_assert!(self.verify(type_registry).is_ok());
                let ty = elements.first().unwrap().typeref(type_registry);
                type_registry.search_or_insert(
                    ArrayType {
                        ty,
                        num_elements: elements.len() as u16,
                    }
                    .into(),
                )
            }
            AnyConst::Struct { elements, packed } => {
                let element_types: Vec<Typeref> = elements
                    .iter()
                    .map(|elem| elem.typeref(type_registry))
                    .collect();
                type_registry.search_or_insert(
                    StructType {
                        element_types,
                        packed: *packed,
                    }
                    .into(),
                )
            }
        }
    }

    /// Format the constant as a string.
    pub fn fmt<'a>(&'a self, module: Option<&'a Module>) -> impl std::fmt::Display + 'a {
        pub struct Fmt<'a> {
            constant: &'a AnyConst,
            module: Option<&'a Module>,
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.constant {
                    AnyConst::Int(ic) => ic.fmt(f),
                    AnyConst::Float(fc) => fc.fmt(f),
                    AnyConst::Array { elements } => {
                        write!(f, "[")?;
                        for (i, elem) in elements.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{:#}", elem.fmt(self.module))?;
                        }
                        write!(f, "]")
                    }
                    AnyConst::Struct { elements, packed } => {
                        if *packed {
                            write!(f, "packed ")?;
                        }
                        write!(f, "{{")?;

                        for (i, elem) in elements.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{:#}", elem.fmt(self.module))?;
                        }
                        write!(f, "}}")
                    }
                    AnyConst::Ptr(pointer) => {
                        if !f.alternate() {
                            write!(f, "ptr ")?;
                        }

                        if let Some(module) = self.module {
                            match module.find_symbol_by_ptr(pointer) {
                                Ok(Symbol::ExternalFunction(external_func)) => {
                                    write!(f, "ptr external {}", external_func.name)
                                }
                                Ok(Symbol::Function(func)) if func.name.is_some() => {
                                    write!(f, "ptr function {}", func.name.as_ref().unwrap())
                                }
                                Ok(Symbol::Global(global)) if global.name.is_some() => {
                                    write!(f, "ptr global {}", global.name.as_ref().unwrap())
                                }
                                Ok(_) => {
                                    write!(f, "@{:?}", pointer.0)
                                }
                                Err(_) => {
                                    write!(f, "ptr <unresolved@{:?}>", pointer)
                                }
                            }
                        } else {
                            write!(f, "ptr <unresolved@{:?}>", pointer)
                        }
                    }
                }
            }
        }

        Fmt {
            constant: self,
            module,
        }
    }
}

impl<T: Into<IConst>> From<T> for AnyConst {
    fn from(value: T) -> Self {
        AnyConst::Int(value.into())
    }
}

impl From<FConst> for AnyConst {
    fn from(value: FConst) -> Self {
        AnyConst::Float(value)
    }
}

impl From<Pointer> for AnyConst {
    fn from(value: Pointer) -> Self {
        AnyConst::Ptr(value)
    }
}
