use crate::{
    expr::{Expr, expr_sealed},
    prop::Prop,
    variable::InlineVariable,
};

/// Represents a variable expr.
///
/// A variable expr is simply a reference to a variable identified by its name.
impl expr_sealed::Sealed for InlineVariable {}
impl Expr for InlineVariable {
    fn dispatch(
        &self,
    ) -> crate::expr::dispatch::ExprDispatch<
        impl crate::prop::Prop,
        impl crate::expr::Expr,
        impl crate::expr::Expr,
    > {
        crate::expr::dispatch::ExprDispatch::<
            crate::prop::DynProp,
            crate::expr::DynExpr,
            crate::expr::DynExpr,
        >::Var(*self)
    }
}

/// Note: Propositions implement `Expr` individually in `prop::defs` to avoid
/// overlapping blanket implementations with references.

/// A dynamic expr that holds whenever a expr is deemed unreachable.
pub struct Unreachable;

impl expr_sealed::Sealed for Unreachable {}
impl Expr for Unreachable {
    fn dispatch(
        &self,
    ) -> crate::expr::dispatch::ExprDispatch<
        impl crate::prop::Prop,
        impl crate::expr::Expr,
        impl crate::expr::Expr,
    > {
        crate::expr::dispatch::ExprDispatch::<
            crate::prop::DynProp,
            crate::expr::DynExpr,
            crate::expr::DynExpr,
        >::Unreachable
    }
}

/// Represents the application of a function to an argument.
///
/// If `f` is a variable representing a function and `A` is a expr representing an argument,
/// then `App<A>` represents the expr `f(A)`.
pub struct App<A: Expr> {
    pub func: InlineVariable,
    pub arg: A,
}

impl<A: Expr> expr_sealed::Sealed for App<A> {}
impl<A: Expr> Expr for App<A> {
    fn dispatch(
        &self,
    ) -> crate::expr::dispatch::ExprDispatch<
        impl crate::prop::Prop,
        impl crate::expr::Expr,
        impl crate::expr::Expr,
    > {
        crate::expr::dispatch::ExprDispatch::<crate::prop::DynProp, &A, crate::expr::DynExpr>::App {
            func: self.func,
            arg: &self.arg,
        }
    }
}

/// Represents a conditional expr.
///
/// If `P` is a proposition, `T` and `E` are exprs, then `If<P, T, E>` represents the expr
/// that evaluates to `T` if `P` is true, and `E` otherwise.
pub struct If<P: Prop, T: Expr, E: Expr> {
    pub condition: P,
    pub then_branch: T,
    pub else_branch: E,
}

impl<P: Prop, T: Expr, E: Expr> expr_sealed::Sealed for If<P, T, E> {}
impl<P: Prop, T: Expr, E: Expr> Expr for If<P, T, E> {
    fn dispatch(
        &self,
    ) -> crate::expr::dispatch::ExprDispatch<
        impl crate::prop::Prop,
        impl crate::expr::Expr,
        impl crate::expr::Expr,
    > {
        crate::expr::dispatch::ExprDispatch::<&P, &T, &E>::If {
            condition: &self.condition,
            then_branch: &self.then_branch,
            else_branch: &self.else_branch,
        }
    }
}

/// Represents a tuple expr (distinct from a type tuple `TTuple`).
///
/// If `A` and `B` are exprs, then `ETuple<A, B>` represents the expr `(A, B)`.
/// Tuples can be nested to create tuples of arbitrary length.
/// For example, `ETuple<A, ETuple<B, C>>` represents the expr `(A, (B, C))`.
///
/// Note that this is a binary tuple, so the second element can be another tuple.
pub struct ETuple<A: Expr, B: Expr> {
    pub first: A,
    pub second: B,
}

impl<A: Expr, B: Expr> expr_sealed::Sealed for ETuple<A, B> {}
impl<A: Expr, B: Expr> Expr for ETuple<A, B> {
    fn dispatch(
        &self,
    ) -> crate::expr::dispatch::ExprDispatch<
        impl crate::prop::Prop,
        impl crate::expr::Expr,
        impl crate::expr::Expr,
    > {
        crate::expr::dispatch::ExprDispatch::<crate::prop::DynProp, &A, &B>::Tuple(
            &self.first,
            &self.second,
        )
    }
}
