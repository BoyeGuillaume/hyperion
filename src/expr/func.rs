use crate::expr::Expr;
use crate::expr::defs::*;
use crate::variable::InlineVariable;

pub fn forall<Q: Expr, R: Expr>(variable: InlineVariable, dtype: Q, inner: R) -> ForAll<Q, R> {
    ForAll {
        variable,
        dtype,
        inner,
    }
}

pub fn exists<Q: Expr, R: Expr>(variable: InlineVariable, dtype: Q, inner: R) -> Exists<Q, R> {
    Exists {
        variable,
        dtype,
        inner,
    }
}

pub fn tuple<A: Expr, B: Expr>(first: A, second: B) -> Tuple<A, B> {
    Tuple { first, second }
}

pub fn lambda<A: Expr, B: Expr>(arg: A, body: B) -> Lambda<A, B> {
    Lambda { arg, body }
}

pub fn call<A: Expr, B: Expr>(func: A, arg: B) -> Call<A, B> {
    Call { func, arg }
}

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

pub fn powerset<A: Expr>(inner: A) -> Powerset<A> {
    Powerset { inner }
}

pub fn equals<A: Expr, B: Expr>(lhs: A, rhs: B) -> Equal<A, B> {
    Equal { lhs, rhs }
}

pub fn and<A: Expr, B: Expr>(lhs: A, rhs: B) -> And<A, B> {
    And { lhs, rhs }
}

pub fn or<A: Expr, B: Expr>(lhs: A, rhs: B) -> Or<A, B> {
    Or { lhs, rhs }
}

pub fn implies<A: Expr, B: Expr>(antecedent: A, consequent: B) -> Implies<A, B> {
    Implies {
        antecedent,
        consequent,
    }
}

pub fn iff<A: Expr, B: Expr>(lhs: A, rhs: B) -> Iff<A, B> {
    Iff { lhs, rhs }
}

pub fn not<A: Expr>(inner: A) -> Not<A> {
    Not { inner }
}
