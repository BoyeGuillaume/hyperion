//! Dispatch enum for the unified expression language.
//!
//! Every node in the language decodes to a single enum with children of the same
//! expression type parameters `E1, E2, E3` (for unary/binary/ternary shapes). This keeps
//! traversals allocation-free and monomorphized for performance.
use strum::EnumIs;

use crate::{
    expr::{Expr, variant::ExprType},
    variable::InlineVariable,
};

/// Describes the outer constructor of an expression and borrows its children.
#[derive(Debug, Clone, Copy, EnumIs)]
pub enum ExprView<E1: Expr, E2: Expr, E3: Expr> {
    // Constant expr
    Bool,
    Omega,
    True,
    False,
    Never,

    // Unary expr
    Not(E1),
    Powerset(E1),

    // Binary expr
    And(E1, E2),
    Or(E1, E2),
    Implies(E1, E2),
    Iff(E1, E2),
    Equal(E1, E2),
    Lambda {
        arg: E1,
        body: E2,
    },
    Call {
        func: E1,
        arg: E2,
    },
    Tuple(E1, E2), // Also used as a type
    Forall {
        variable: InlineVariable,
        dtype: E1,
        inner: E2,
    },
    Exists {
        variable: InlineVariable,
        dtype: E1,
        inner: E2,
    },

    // Ternary expr
    If {
        condition: E1,
        then_branch: E2,
        else_branch: E3,
    },

    // Misc
    Variable(InlineVariable),
}

impl<E1: Expr, E2: Expr, E3: Expr> ExprView<E1, E2, E3> {
    /// Return the discriminant identifying the kind of this node.
    pub fn type_(&self) -> ExprType {
        pub use ExprView::*;

        match self {
            Bool => ExprType::Bool,
            Omega => ExprType::Omega,
            True => ExprType::True,
            False => ExprType::False,
            Never => ExprType::Never,
            Not(_) => ExprType::Not,
            Powerset(_) => ExprType::Powerset,
            And(_, _) => ExprType::And,
            Or(_, _) => ExprType::Or,
            Implies(_, _) => ExprType::Implies,
            Iff(_, _) => ExprType::Iff,
            Equal(_, _) => ExprType::Equal,
            Lambda { .. } => ExprType::Lambda,
            Call { .. } => ExprType::Call,
            Tuple(_, _) => ExprType::Tuple,
            Forall { .. } => ExprType::Forall,
            Exists { .. } => ExprType::Exists,
            If { .. } => ExprType::If,
            Variable(_) => ExprType::Variable,
        }
    }

    #[inline]
    /// Alias for [`type_`], useful when `type` is a reserved word.
    pub fn r#type(&self) -> ExprType {
        self.type_()
    }
}
