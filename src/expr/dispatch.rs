//! Dispatch enum describing the outer shape of an expression.
//!
//! Produced by [`crate::expr::Expr::decode_expr`], parameterized over the concrete child
//! types to return for propositions and expressions.
use strum::{EnumDiscriminants, EnumIs};

use crate::{expr::Expr, prop::Prop, variable::InlineVariable};

/// Describes the outer constructor of an expression and borrows its children.
#[derive(Debug, Clone, Copy, EnumIs, EnumDiscriminants)]
#[strum_discriminants(derive(PartialOrd, Ord, Hash))]
#[strum_discriminants(name(ExprDispatchVariant))]
#[strum_discriminants(vis(pub(crate)))]
pub enum ExprView<P: Prop, T1: Expr, T2: Expr> {
    /// Variable reference.
    Var(InlineVariable),
    /// Unreachable expression.
    Unreachable,
    /// Application `func(arg)`.
    App {
        /// Function variable identifier.
        func: InlineVariable,
        /// Argument expression.
        arg: T1,
    },
    /// Conditional `if condition { then_branch } else { else_branch }`.
    If {
        /// Condition proposition.
        condition: P,
        /// Then branch evaluated when condition holds.
        then_branch: T1,
        /// Else branch evaluated when condition does not hold.
        else_branch: T2,
    },
    /// Tuple `(left, right)`.
    Tuple(T1, T2),
    /// Embedded proposition as an expression.
    Prop(P),
}
