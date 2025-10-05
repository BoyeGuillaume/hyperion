//! Dispatch enum for the unified expression language.
//!
//! Every node in the language decodes to a single enum with children of the same
//! expression type parameter `E`.
use strum::{EnumDiscriminants, EnumIs};

use crate::variable::InlineVariable;

/// Describes the outer constructor of an expression and borrows its children.
#[derive(Debug, Clone, Copy, EnumIs, EnumDiscriminants)]
#[strum_discriminants(derive(PartialOrd, Ord, Hash))]
#[strum_discriminants(name(ExprDispatchVariant))]
#[strum_discriminants(vis(pub))]
pub enum ExprView<E1, E2, E3> {
    // Term-level
    Var(InlineVariable),
    Unreachable,
    App {
        func: InlineVariable,
        arg: E1,
    },
    If {
        condition: E1,
        then_branch: E2,
        else_branch: E3,
    },
    Tuple(E1, E2), // Also used as a type

    // Logic-level
    True,
    False,
    Not(E1),
    And(E1, E2),
    Or(E1, E2),
    Implies(E1, E2),
    Iff(E1, E2),
    ForAll {
        variable: InlineVariable,
        dtype: E1,
        inner: E2,
    },
    Exists {
        variable: InlineVariable,
        dtype: E1,
        inner: E2,
    },
    Equal(E1, E2),

    // Type-level
    Bool,
    Omega,
    Never,
    Powerset(E1),
    Func(E1, E2),
}
