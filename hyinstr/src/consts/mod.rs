//! Constant values
//!
//! Literal values used as immediate operands in instructions. Both integer
//! and floating‑point constants are supported, with arbitrary precision types
//! where appropriate.
use crate::{
    consts::{fp::FConst, int::IConst},
    modules::{Module, symbol::FunctionPointer},
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
use uuid::Uuid;

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

    /// Function pointer constant (should be used only for function call instructions)
    FuncPtr(FunctionPointer),

    /// Global pointer constant (should be used only for global load/store instructions)
    GlobalPtr(Uuid),
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
            AnyConst::Struct { .. } => Ok(()), // No restriction on struct element types
            _ => Ok(()),
        }
    }

    /// Retrieve the type of the constant.
    pub fn typeref(&self, type_registry: &TypeRegistry) -> Typeref {
        match self {
            AnyConst::Int(ic) => type_registry.search_or_insert(ic.ty.into()),
            AnyConst::Float(fc) => type_registry.search_or_insert(fc.ty.into()),
            AnyConst::FuncPtr(_) | AnyConst::GlobalPtr(_) => {
                type_registry.search_or_insert(PtrType.into())
            }
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
                debug_assert!(self.verify(type_registry).is_ok());
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
                    AnyConst::FuncPtr(fp) => match fp {
                        FunctionPointer::Internal(uuid) => {
                            if !f.alternate() {
                                write!(f, "ptr ")?;
                            }

                            if let Some(module) = self.module {
                                if let Some(func) = module.functions.get(uuid) {
                                    if let Some(name) = &func.name {
                                        write!(f, "{}", name)
                                    } else {
                                        write!(f, "@{:?}", uuid)
                                    }
                                } else {
                                    write!(f, "<invalid@{:?}>", uuid)
                                }
                            } else {
                                write!(f, "<unresolved@{:?}>", uuid)
                            }
                        }
                        FunctionPointer::External(name) => {
                            if let Some(module) = self.module {
                                if let Some(func) = module.external_functions.get(name) {
                                    write!(f, "ptr external {}", func.name)
                                } else {
                                    write!(f, "ptr external <invalid@{}>", name)
                                }
                            } else {
                                write!(f, "ptr external <unresolved@{}>", name)
                            }
                        }
                    },
                    AnyConst::GlobalPtr(uuid) => {
                        if let Some(module) = self.module {
                            if let Some(global) = module.globals.get(uuid) {
                                if let Some(name) = &global.name {
                                    write!(f, "{}", name)
                                } else {
                                    write!(f, "@{:?}", uuid)
                                }
                            } else {
                                write!(f, "ptr <invalid@{}>", uuid)
                            }
                        } else {
                            write!(f, "ptr <unresolved@{}>", uuid)
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

impl From<FunctionPointer> for AnyConst {
    fn from(value: FunctionPointer) -> Self {
        AnyConst::FuncPtr(value)
    }
}
