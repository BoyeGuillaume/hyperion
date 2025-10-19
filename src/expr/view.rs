//! Dispatch enum for the unified expression language.
//!
//! Every node in the language decodes to a single enum with children of the same
//! expression type parameters `E1, E2, E3` (for unary/binary/ternary shapes). This keeps
//! traversals allocation-free and monomorphized for performance.
use strum::EnumIs;

use crate::{expr::variant::ExprType, variable::InlineVariable};

/// Describes the outer constructor of an expression and borrows its children.
#[derive(Debug, Clone, Copy, EnumIs)]
pub enum ExprView<E1, E2, E3> {
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

impl<E1, E2, E3> ExprView<E1, E2, E3> {
    /// Return the discriminant identifying the kind of this node.
    #[inline]
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

    /// Alias for [`type_`], useful when `type` is a reserved word.
    #[inline]
    pub fn r#type(&self) -> ExprType {
        self.type_()
    }

    /// Map the children of this expression view to new values.
    #[inline]
    pub fn map<F1, F2, F3, O1, O2, O3>(self, f1: F1, f2: F2, f3: F3) -> ExprView<O1, O2, O3>
    where
        F1: FnOnce(E1) -> O1,
        F2: FnOnce(E2) -> O2,
        F3: FnOnce(E3) -> O3,
    {
        pub use ExprView::*;

        match self {
            Bool => Bool,
            Omega => Omega,
            True => True,
            False => False,
            Never => Never,
            Not(e) => Not(f1(e)),
            Powerset(e) => Powerset(f1(e)),
            And(e1, e2) => And(f1(e1), f2(e2)),
            Or(e1, e2) => Or(f1(e1), f2(e2)),
            Implies(e1, e2) => Implies(f1(e1), f2(e2)),
            Iff(e1, e2) => Iff(f1(e1), f2(e2)),
            Equal(e1, e2) => Equal(f1(e1), f2(e2)),
            Lambda { arg, body } => Lambda {
                arg: f1(arg),
                body: f2(body),
            },
            Call { func, arg } => Call {
                func: f1(func),
                arg: f2(arg),
            },
            Tuple(e1, e2) => Tuple(f1(e1), f2(e2)),
            Forall {
                variable,
                dtype,
                inner,
            } => Forall {
                variable,
                dtype: f1(dtype),
                inner: f2(inner),
            },
            Exists {
                variable,
                dtype,
                inner,
            } => Exists {
                variable,
                dtype: f1(dtype),
                inner: f2(inner),
            },
            If {
                condition,
                then_branch,
                else_branch,
            } => If {
                condition: f1(condition),
                then_branch: f2(then_branch),
                else_branch: f3(else_branch),
            },
            Variable(v) => Variable(v),
        }
    }
}

impl<E1> ExprView<E1, E1, E1> {
    /// Map the children of this expression view to new values.
    #[inline]
    pub fn map_unary<F, O>(self, f: F) -> ExprView<O, O, O>
    where
        F: FnOnce(E1, u8) -> O + Copy,
    {
        self.map(|e| f(e, 0), |e| f(e, 1), |e| f(e, 2))
    }
}
