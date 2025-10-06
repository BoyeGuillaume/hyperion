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

impl ExprDispatchVariant {
    /// Returns true if this variant can represent a type.
    /// Note: Tuple can be a type as well as an expression.
    pub fn can_be_type(&self) -> bool {
        use ExprDispatchVariant::*;
        matches!(self, Bool | Omega | Never | Powerset | Func | Tuple | Var)
    }

    /// Returns true if this variant represents a proposition.
    /// Propositions are a subset of expressions.
    pub fn can_be_prop(&self) -> bool {
        use ExprDispatchVariant::*;
        matches!(
            self,
            True | False | Not | And | Or | Implies | Iff | ForAll | Exists | Equal | Var
        )
    }

    /// Returns true if this variant can represent a term-level expression.
    /// Note: all propositions are expressions, and Tuple can also be an expression.
    pub fn can_be_expr(&self) -> bool {
        use ExprDispatchVariant::*;
        matches!(
            self,
            // Term-level
            Var | Never | App | If | Tuple |
            // Logic-level (propositions are expressions)
            True | False | Not | And | Or | Implies | Iff | ForAll | Exists | Equal
        )
    }
}

impl<E1, E2, E3> ExprView<E1, E2, E3> {
    /// Returns true if this expression can represent a type.
    pub fn can_be_type(&self) -> bool {
        ExprDispatchVariant::from(self).can_be_type()
    }

    /// Returns true if this expression represents a proposition.
    pub fn can_be_prop(&self) -> bool {
        ExprDispatchVariant::from(self).can_be_prop()
    }

    /// Returns true if this is a (term-level) expression.
    /// Propositions are expressions; Tuple can be both an expression and a type.
    pub fn can_be_expr(&self) -> bool {
        ExprDispatchVariant::from(self).can_be_expr()
    }
}
