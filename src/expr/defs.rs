use crate::{
    encoding::{DynBuf, RawEncodable, integer::encode_u64, magic, push_len},
    expr::{Expr, dispatch::ExprDispatch, expr_sealed},
    prop::{DynProp, Prop},
    variable::InlineVariable,
};

/// Represents a variable expr.
///
/// A variable expr is simply a reference to a variable identified by its name.
impl expr_sealed::Sealed for InlineVariable {}
impl Expr for InlineVariable {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<DynProp, crate::expr::DynExpr, crate::expr::DynExpr>::Var(*self)
    }
}

/// Note: Propositions implement `Expr` individually in `prop::defs` to avoid
/// overlapping blanket implementations with references.

/// A dynamic expr that holds whenever a expr is deemed unreachable.
pub struct Unreachable;

impl expr_sealed::Sealed for Unreachable {}
impl Expr for Unreachable {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<DynProp, crate::expr::DynExpr, crate::expr::DynExpr>::Unreachable
    }
}

impl RawEncodable for Unreachable {
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.push(magic::E_UNREACHABLE);
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
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<DynProp, &A, crate::expr::DynExpr>::App {
            func: self.func,
            arg: &self.arg,
        }
    }
}

impl<A: Expr + RawEncodable> RawEncodable for App<A> {
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.arg.encode_raw(buf);
        // func id payload
        encode_u64(self.func.id(), buf);
        buf.push(magic::E_APP);
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
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&P, &T, &E>::If {
            condition: &self.condition,
            then_branch: &self.then_branch,
            else_branch: &self.else_branch,
        }
    }
}

impl<P, T, E> RawEncodable for If<P, T, E>
where
    P: Prop + RawEncodable,
    T: Expr + RawEncodable,
    E: Expr + RawEncodable,
{
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.condition.encode_raw(buf);
        let then_start = buf.len();
        self.then_branch.encode_raw(buf);
        let then_len = buf.len() - then_start;
        let else_start = buf.len();
        self.else_branch.encode_raw(buf);
        let else_len = buf.len() - else_start;
        push_len(else_len, buf);
        push_len(then_len, buf);
        buf.push(magic::E_IF);
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
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<DynProp, &A, &B>::Tuple(&self.first, &self.second)
    }
}

impl<A: Expr + RawEncodable, B: Expr + RawEncodable> RawEncodable for ETuple<A, B> {
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.first.encode_raw(buf);
        let right_start = buf.len();
        self.second.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::E_TUPLE);
    }
}
