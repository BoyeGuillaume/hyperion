//! Constant values
//!
//! Literal values used as immediate operands in instructions. Both integer
//! and floating‑point constants are supported, with arbitrary precision types
//! where appropriate.
use crate::{
    consts::{fp::FConst, int::IConst},
    modules::{Module, symbol::FunctionPointer},
    types::{TypeRegistry, Typeref, primary::PtrType},
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

    /// Function pointer constant (should be used only for function call instructions)
    FuncPtr(FunctionPointer),
}

impl AnyConst {
    /// Retrieve the type of the constant.
    pub fn typeref(&self, type_registry: &TypeRegistry) -> Typeref {
        match self {
            AnyConst::Int(ic) => type_registry.search_or_insert(ic.ty.into()),
            AnyConst::Float(fc) => type_registry.search_or_insert(fc.ty.into()),
            AnyConst::FuncPtr(_) => type_registry.search_or_insert(PtrType.into()),
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
                    AnyConst::Int(ic) => write!(f, "{}", ic),
                    AnyConst::Float(fc) => write!(f, "{}", fc),
                    AnyConst::FuncPtr(fp) => match fp {
                        FunctionPointer::Internal(uuid) => {
                            if let Some(module) = self.module {
                                if let Some(func) = module.functions.get(uuid) {
                                    if let Some(name) = &func.name {
                                        write!(f, "ptr %{}", name)
                                    } else {
                                        write!(f, "ptr @{:?}", uuid)
                                    }
                                } else {
                                    write!(f, "ptr <invalid@{:?}>", uuid)
                                }
                            } else {
                                write!(f, "ptr <unresolved@{:?}>", uuid)
                            }
                        }
                        FunctionPointer::External(name) => {
                            if let Some(module) = self.module {
                                if let Some(func) = module.external_functions.get(name) {
                                    write!(f, "ptr external %{}", func.name)
                                } else {
                                    write!(f, "ptr external <invalid@{}>", name)
                                }
                            } else {
                                write!(f, "ptr external <unresolved@{}>", name)
                            }
                        }
                    },
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
