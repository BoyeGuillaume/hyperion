//! Builder-style helper functions to construct expressions.
//!
//! These mirror the methods available on the [`Expr`](crate::expr::Expr) trait but can be used as
//! free functions. They are zero-cost wrappers returning lightweight node structs.
use crate::expr::Expr;
use crate::expr::defs::*;
use crate::variable::InlineVariable;

/// Universal quantification: `forall x : dtype . inner`.
pub fn forall<Q: Expr, R: Expr>(variable: InlineVariable, dtype: Q, inner: R) -> ForAll<Q, R> {
    ForAll {
        variable,
        dtype,
        inner,
    }
}

/// Existential quantification: `exists x : dtype . inner`.
pub fn exists<Q: Expr, R: Expr>(variable: InlineVariable, dtype: Q, inner: R) -> Exists<Q, R> {
    Exists {
        variable,
        dtype,
        inner,
    }
}

/// Pair `(first, second)`.
pub fn tuple<A: Expr, B: Expr>(first: A, second: B) -> Tuple<A, B> {
    Tuple { first, second }
}

/// Lambda abstraction `arg -> body`.
pub fn lambda<A: Expr, B: Expr>(arg: A, body: B) -> Lambda<A, B> {
    Lambda { arg, body }
}

/// Function call `func(arg)`.
pub fn call<A: Expr, B: Expr>(func: A, arg: B) -> Call<A, B> {
    Call { func, arg }
}

/// Conditional `if condition then then_branch else else_branch`.
pub fn branch<A: Expr, B: Expr, C: Expr>(
    condition: A,
    then_branch: B,
    else_branch: C,
) -> If<A, B, C> {
    If {
        condition,
        then_branch,
        else_branch,
    }
}

/// Powerset type `P(inner)`.
pub fn powerset<A: Expr>(inner: A) -> Powerset<A> {
    Powerset { inner }
}

/// Equality `lhs = rhs`.
pub fn equals<A: Expr, B: Expr>(lhs: A, rhs: B) -> Equal<A, B> {
    Equal { lhs, rhs }
}

/// Conjunction `lhs /\ rhs`.
pub fn and<A: Expr, B: Expr>(lhs: A, rhs: B) -> And<A, B> {
    And { lhs, rhs }
}

/// Disjunction `lhs \/ rhs`.
pub fn or<A: Expr, B: Expr>(lhs: A, rhs: B) -> Or<A, B> {
    Or { lhs, rhs }
}

/// Implication `antecedent => consequent`.
pub fn implies<A: Expr, B: Expr>(antecedent: A, consequent: B) -> Implies<A, B> {
    Implies {
        antecedent,
        consequent,
    }
}

/// Equivalence `lhs <=> rhs`.
pub fn iff<A: Expr, B: Expr>(lhs: A, rhs: B) -> Iff<A, B> {
    Iff { lhs, rhs }
}

/// Negation `!inner`.
pub fn not<A: Expr>(inner: A) -> Not<A> {
    Not { inner }
}
