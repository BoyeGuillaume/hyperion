//! Concrete expression constructors used to build expressions.
//!
//! These types implement [`crate::expr::Expr`] and support encoding/decoding.
use crate::{
    encoding::{DynBuf, RawEncodable, integer::encode_u64, magic, push_len},
    expr::{Expr, dispatch::ExprDispatch, expr_sealed},
    prop::{DynProp, Prop},
    variable::InlineVariable,
};

/// Variable expression referencing an inline variable identifier.
impl expr_sealed::Sealed for InlineVariable {}
impl Expr for InlineVariable {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<DynProp, crate::expr::DynExpr, crate::expr::DynExpr>::Var(*self)
    }
}

/// Note: Propositions implement `Expr` individually in `prop::defs` to avoid
/// overlapping blanket implementations with references.

/// An expression that denotes unreachable code (no value can be produced).
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

/// Application of a function variable to an argument: `f(arg)`.
pub struct App<A: Expr> {
    /// Function variable identifier.
    pub func: InlineVariable,
    /// Argument expression.
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

impl<A: Expr> RawEncodable for App<A> {
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.arg.encode_raw(buf);
        // func id payload
        encode_u64(self.func.raw(), buf);
        buf.push(magic::E_APP);
    }
}

/// Conditional expression `if condition { then } else { else }`.
pub struct If<P: Prop, T: Expr, E: Expr> {
    /// Condition to test.
    pub condition: P,
    /// Expression evaluated when `condition` holds.
    pub then_branch: T,
    /// Expression evaluated otherwise.
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

impl<P: Prop, T: Expr, E: Expr> RawEncodable for If<P, T, E> {
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

/// Tuple expression `(A, B)` (binary; can be nested for longer tuples).
pub struct ETuple<A: Expr, B: Expr> {
    /// First component.
    pub first: A,
    /// Second component.
    pub second: B,
}

impl<A: Expr, B: Expr> expr_sealed::Sealed for ETuple<A, B> {}
impl<A: Expr, B: Expr> Expr for ETuple<A, B> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<DynProp, &A, &B>::Tuple(&self.first, &self.second)
    }
}

impl<A: Expr, B: Expr> RawEncodable for ETuple<A, B> {
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.first.encode_raw(buf);
        let right_start = buf.len();
        self.second.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::E_TUPLE);
    }
}
