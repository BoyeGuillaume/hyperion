//! Dispatch enum describing the outer shape of a type.
//!
//! Produced by [`crate::dtype::DType::decode_dtype`], with type parameters recording the
//! concrete child types that the caller wishes to receive.
use strum::{EnumDiscriminants, EnumIs};

use crate::{dtype::DType, variable::InlineVariable};

/// Describes the outer constructor of a type and borrows its children.
#[derive(Debug, Clone, Copy, EnumIs, EnumDiscriminants)]
#[strum_discriminants(derive(PartialOrd, Ord, Hash))]
#[strum_discriminants(name(DTypeDispatchVariant))]
#[strum_discriminants(vis(pub(crate)))]
pub enum DTypeDispatch<T1: DType, T2: DType> {
    /// Boolean type
    Bool,
    /// The universe of all well-formed types (type of types)
    Omega,
    /// The uninhabited type (never)
    Never,
    /// Type variable
    Var(InlineVariable),
    /// Function type `T1 -> T2`.
    Arrow(T1, T2),
    /// Product type `T1 x T2`.
    Tuple(T1, T2),
    /// Powerset type `P(T1)`.
    Power(T1),
}
