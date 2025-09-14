//! Dispatch enum describing the outer shape of a proposition.
//!
//! Produced by [`crate::prop::Prop::decode_prop`] and parameterized by concrete
//! child types for propositions, expressions, and types.
use strum::{EnumDiscriminants, EnumIs};

use crate::{dtype::DType, expr::Expr, prop::Prop, variable::InlineVariable};

/// Describes the outer constructor of a proposition and borrows its children.
#[derive(Debug, Clone, Copy, EnumIs, EnumDiscriminants)]
#[strum_discriminants(derive(PartialOrd, Ord, Hash))]
#[strum_discriminants(name(PropDispatchVariant))]
#[strum_discriminants(vis(pub(crate)))]
pub enum PropDispatch<P1: Prop, P2: Prop, T1: Expr, T2: Expr, DT: DType> {
    /// True.
    True,
    /// False.
    False,
    /// Negation.
    Not(P1),
    /// Conjunction.
    And(P1, P2),
    /// Disjunction.
    Or(P1, P2),
    /// Implication.
    Implies(P1, P2),
    /// Biconditional.
    Iff(P1, P2),
    /// Universal quantification over a variable of type `DT`.
    ForAll {
        /// Bound variable id.
        variable: InlineVariable,
        /// Domain type.
        dtype: DT,
        /// Inner proposition.
        inner: P1,
    },
    /// Existential quantification over a variable of type `DT`.
    Exists {
        /// Bound variable id.
        variable: InlineVariable,
        /// Domain type.
        dtype: DT,
        /// Inner proposition.
        inner: P1,
    },
    /// Equality of two expressions.
    Equal(T1, T2),
}
