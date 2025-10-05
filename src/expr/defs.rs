//! Concrete expression constructors used to build expressions.
//!
//! These types implement [`crate::expr::Expr`] and support encoding/decoding.
use crate::{
    encoding::{
        RawEncodable,
        integer::{encode_u64, encoded_size_u64},
        magic,
    },
    expr::{Expr, view::ExprView, expr_sealed},
    prop::{DynProp, Prop},
    variable::InlineVariable,
};

/// Variable expression referencing an inline variable identifier.
impl expr_sealed::Sealed for InlineVariable {}
impl Expr for InlineVariable {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<DynProp, crate::expr::DynExpr, crate::expr::DynExpr>::Var(*self)
    }
}

/// Note: Propositions implement `Expr` individually in `prop::defs` to avoid
/// overlapping blanket implementations with references.

/// An expression that denotes unreachable code (no value can be produced).
pub struct Unreachable;

impl expr_sealed::Sealed for Unreachable {}
impl Expr for Unreachable {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<DynProp, crate::expr::DynExpr, crate::expr::DynExpr>::Unreachable
    }
}

impl RawEncodable for Unreachable {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[magic::E_UNREACHABLE]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
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
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<DynProp, &A, crate::expr::DynExpr>::App {
            func: self.func,
            arg: &self.arg,
        }
    }
}

impl<A: Expr> RawEncodable for App<A> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.arg.encode_raw(f);
        // func id payload
        size += encode_u64(self.func.raw(), f);
        f(&[magic::E_APP]);

        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.arg.encoded_size() + encoded_size_u64(self.func.raw()) + 1
    }
}

impl<A: Expr> App<A> {
    /// Substitute the argument expression while keeping the same function.
    pub fn subs_arg<R: Expr>(self, arg: R) -> App<R> {
        App {
            func: self.func,
            arg,
        }
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
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&P, &T, &E>::If {
            condition: &self.condition,
            then_branch: &self.then_branch,
            else_branch: &self.else_branch,
        }
    }
}

impl<P: Prop, T: Expr, E: Expr> RawEncodable for If<P, T, E> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.condition.encode_raw(f);
        let then_len = self.then_branch.encode_raw(f);
        size += then_len;

        let else_len = self.else_branch.encode_raw(f);
        size += else_len;

        size += encode_u64(else_len as u64, f);
        size += encode_u64(then_len as u64, f);
        f(&[magic::E_IF]);

        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.condition.encoded_size()
            + self.then_branch.encoded_size()
            + self.else_branch.encoded_size()
            + encoded_size_u64(self.then_branch.encoded_size())
            + encoded_size_u64(self.else_branch.encoded_size())
            + 1
    }
}

impl<P: Prop, T: Expr, E: Expr> If<P, T, E> {
    /// Substitute the condition while keeping the same branches.
    pub fn subs_condition<R: Prop>(self, condition: R) -> If<R, T, E> {
        If {
            condition,
            then_branch: self.then_branch,
            else_branch: self.else_branch,
        }
    }

    /// Substitute the then-branch while keeping the same condition and else-branch.
    pub fn subs_then<R: Expr>(self, then_branch: R) -> If<P, R, E> {
        If {
            condition: self.condition,
            then_branch,
            else_branch: self.else_branch,
        }
    }

    /// Substitute the else-branch while keeping the same condition and then-branch.
    pub fn subs_else<R: Expr>(self, else_branch: R) -> If<P, T, R> {
        If {
            condition: self.condition,
            then_branch: self.then_branch,
            else_branch,
        }
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
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<DynProp, &A, &B>::Tuple(&self.first, &self.second)
    }
}

impl<A: Expr, B: Expr> RawEncodable for ETuple<A, B> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.first.encode_raw(f);

        let right_len = self.second.encode_raw(f);
        size += right_len;

        size += encode_u64(right_len as u64, f);
        f(&[magic::E_TUPLE]);

        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.first.encoded_size()
            + self.second.encoded_size()
            + encoded_size_u64(self.second.encoded_size())
            + 1
    }
}

impl<A: Expr, B: Expr> ETuple<A, B> {
    /// Substitute the first component while keeping the second.
    pub fn subs_first<R: Expr>(self, first: R) -> ETuple<R, B> {
        ETuple {
            first,
            second: self.second,
        }
    }

    /// Substitute the second component while keeping the first.
    pub fn subs_second<R: Expr>(self, second: R) -> ETuple<A, R> {
        ETuple {
            first: self.first,
            second,
        }
    }
}
