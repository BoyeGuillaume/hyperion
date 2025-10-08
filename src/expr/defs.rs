//! Concrete unified expression constructors: terms, logic, and types.
//!
//! All these types implement [`crate::expr::Expr`] and support encoding/decoding.
use crate::{
    encoding::{
        LegacyRawEncodable,
        integer::{encode_u64, encoded_size_u64},
        legacy_magic,
    },
    expr::{AnyExpr, Expr, expr_sealed, view::ExprView},
    variable::InlineVariable,
};

// Lightweight operator sugar for logical combinations on expressions.
macro_rules! define_ops_expr {
    (
        $name:ident
        $( <
            $( $($lft:lifetime),+ $(,)? )?
            $( $($gen_name:ident: $gen:tt ),+ $(,)? )?
        > )?
    ) => {
        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
            _O1: Expr
        > std::ops::BitAnd<_O1> for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = And<Self, _O1>;

            fn bitand(self, rhs: _O1) -> Self::Output {
                And { left: self, right: rhs }
            }
        }

        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
            _O1: Expr
        > std::ops::BitOr<_O1> for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = Or<Self, _O1>;

            fn bitor(self, rhs: _O1) -> Self::Output {
                Or { left: self, right: rhs }
            }
        }

        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
        > std::ops::Not for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = Not<Self>;

            fn not(self) -> Self::Output {
                Not { inner: self }
            }
        }
    };
}

/// Variable expression referencing an inline variable identifier.
impl expr_sealed::Sealed for InlineVariable {}
impl Expr for InlineVariable {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Var(*self)
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
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, AnyExpr, AnyExpr>::App {
            func: self.func,
            arg: &self.arg,
        }
    }
}

impl<A: Expr> LegacyRawEncodable for App<A> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.arg.encode_raw(f);
        // func id payload
        size += encode_u64(self.func.raw(), f);
        f(&[legacy_magic::E_APP]);

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
pub struct If<P: Expr, T: Expr, E: Expr> {
    /// Condition to test.
    pub condition: P,
    /// Expression evaluated when `condition` holds.
    pub then_branch: T,
    /// Expression evaluated otherwise.
    pub else_branch: E,
}

impl<P: Expr, T: Expr, E: Expr> expr_sealed::Sealed for If<P, T, E> {}
impl<P: Expr, T: Expr, E: Expr> Expr for If<P, T, E> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &T, &E>::If {
            condition: &self.condition,
            then_branch: &self.then_branch,
            else_branch: &self.else_branch,
        }
    }
}

impl<P: Expr, T: Expr, E: Expr> LegacyRawEncodable for If<P, T, E> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.condition.encode_raw(f);
        let then_len = self.then_branch.encode_raw(f);
        size += then_len;

        let else_len = self.else_branch.encode_raw(f);
        size += else_len;

        size += encode_u64(else_len as u64, f);
        size += encode_u64(then_len as u64, f);
        f(&[legacy_magic::E_IF]);

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

impl<P: Expr, T: Expr, E: Expr> If<P, T, E> {
    /// Substitute the condition while keeping the same branches.
    pub fn subs_condition<R: Expr>(self, condition: R) -> If<R, T, E> {
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
pub struct Tuple<A: Expr, B: Expr> {
    /// First component.
    pub first: A,
    /// Second component.
    pub second: B,
}

impl<A: Expr, B: Expr> expr_sealed::Sealed for Tuple<A, B> {}
impl<A: Expr, B: Expr> Expr for Tuple<A, B> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, &B, AnyExpr>::Tuple(&self.first, &self.second)
    }
}

impl<A: Expr, B: Expr> LegacyRawEncodable for Tuple<A, B> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.first.encode_raw(f);

        let right_len = self.second.encode_raw(f);
        size += right_len;

        size += encode_u64(right_len as u64, f);
        f(&[legacy_magic::E_TUPLE]);

        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.first.encoded_size()
            + self.second.encoded_size()
            + encoded_size_u64(self.second.encoded_size())
            + 1
    }
}

impl<A: Expr, B: Expr> Tuple<A, B> {
    /// Substitute the first component while keeping the second.
    pub fn subs_first<R: Expr>(self, first: R) -> Tuple<R, B> {
        Tuple {
            first,
            second: self.second,
        }
    }

    /// Substitute the second component while keeping the first.
    pub fn subs_second<R: Expr>(self, second: R) -> Tuple<A, R> {
        Tuple {
            first: self.first,
            second,
        }
    }
}

// ========================= Logic (as expressions) =========================

/// True.
pub struct True;

impl expr_sealed::Sealed for True {}
impl Expr for True {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::True
    }
}

impl LegacyRawEncodable for True {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[legacy_magic::P_TRUE]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

define_ops_expr! { True }

/// False.
pub struct False;

impl expr_sealed::Sealed for False {}
impl Expr for False {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::False
    }
}

impl LegacyRawEncodable for False {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[legacy_magic::P_FALSE]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

define_ops_expr! { False }

/// Negation of an expression (intended for logical use).
pub struct Not<P: Expr> {
    pub inner: P,
}

impl<P: Expr> expr_sealed::Sealed for Not<P> {}
impl<P: Expr> Expr for Not<P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, AnyExpr, AnyExpr>::Not(&self.inner)
    }
}

impl<P: Expr + LegacyRawEncodable> LegacyRawEncodable for Not<P> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let s = self.inner.encode_raw(f);
        f(&[legacy_magic::P_NOT]);
        s + 1
    }

    fn encoded_size(&self) -> u64 {
        self.inner.encoded_size() + 1
    }
}

impl<P: Expr> Not<P> {
    pub fn subs_inner<R: Expr>(self, inner: R) -> Not<R> {
        Not { inner }
    }
}

define_ops_expr! { Not<P: Expr> }

/// Conjunction.
pub struct And<P: Expr, Q: Expr> {
    pub left: P,
    pub right: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for And<P, Q> {}
impl<P: Expr, Q: Expr> Expr for And<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::And(&self.left, &self.right)
    }
}

impl<P: Expr + LegacyRawEncodable, Q: Expr + LegacyRawEncodable> LegacyRawEncodable for And<P, Q> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;
        size += self.left.encode_raw(f);
        let rlen = self.right.encode_raw(f);
        size += rlen;
        size += encode_u64(rlen, f);
        f(&[legacy_magic::P_AND]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.left.encoded_size()
            + self.right.encoded_size()
            + encoded_size_u64(self.right.encoded_size())
            + 1
    }
}

impl<P: Expr, Q: Expr> And<P, Q> {
    pub fn subs_left<R: Expr>(self, left: R) -> And<R, Q> {
        And {
            left,
            right: self.right,
        }
    }

    pub fn subs_right<R: Expr>(self, right: R) -> And<P, R> {
        And {
            left: self.left,
            right,
        }
    }
}

define_ops_expr! { And<P: Expr, Q: Expr> }

/// Disjunction.
pub struct Or<P: Expr, Q: Expr> {
    pub left: P,
    pub right: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for Or<P, Q> {}
impl<P: Expr, Q: Expr> Expr for Or<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Or(&self.left, &self.right)
    }
}

impl<P: Expr + LegacyRawEncodable, Q: Expr + LegacyRawEncodable> LegacyRawEncodable for Or<P, Q> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut s = 0;
        s += self.left.encode_raw(f);
        let r = self.right.encode_raw(f);
        s += r;
        s += encode_u64(r, f);
        f(&[legacy_magic::P_OR]);
        s + 1
    }

    fn encoded_size(&self) -> u64 {
        self.left.encoded_size()
            + self.right.encoded_size()
            + encoded_size_u64(self.right.encoded_size())
            + 1
    }
}

impl<P: Expr, Q: Expr> Or<P, Q> {
    pub fn subs_left<R: Expr>(self, left: R) -> Or<R, Q> {
        Or {
            left,
            right: self.right,
        }
    }

    pub fn subs_right<R: Expr>(self, right: R) -> Or<P, R> {
        Or {
            left: self.left,
            right,
        }
    }
}

define_ops_expr! { Or<P: Expr, Q: Expr> }

/// Implication.
pub struct Implies<P: Expr, Q: Expr> {
    pub antecedent: P,
    pub consequent: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for Implies<P, Q> {}
impl<P: Expr, Q: Expr> Expr for Implies<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Implies(&self.antecedent, &self.consequent)
    }
}

impl<P: Expr + LegacyRawEncodable, Q: Expr + LegacyRawEncodable> LegacyRawEncodable
    for Implies<P, Q>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut s = 0;
        s += self.antecedent.encode_raw(f);
        let r = self.consequent.encode_raw(f);
        s += r;
        s += encode_u64(r, f);
        f(&[legacy_magic::P_IMPLIES]);
        s + 1
    }

    fn encoded_size(&self) -> u64 {
        self.antecedent.encoded_size()
            + self.consequent.encoded_size()
            + encoded_size_u64(self.consequent.encoded_size())
            + 1
    }
}

impl<P: Expr, Q: Expr> Implies<P, Q> {
    pub fn subs_antecedent<R: Expr>(self, antecedent: R) -> Implies<R, Q> {
        Implies {
            antecedent,
            consequent: self.consequent,
        }
    }
    pub fn subs_consequent<R: Expr>(self, consequent: R) -> Implies<P, R> {
        Implies {
            antecedent: self.antecedent,
            consequent,
        }
    }
}

define_ops_expr! { Implies<P: Expr, Q: Expr> }

/// Biconditional.
pub struct Iff<P: Expr, Q: Expr> {
    pub left: P,
    pub right: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for Iff<P, Q> {}
impl<P: Expr, Q: Expr> Expr for Iff<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Iff(&self.left, &self.right)
    }
}

impl<P: Expr + LegacyRawEncodable, Q: Expr + LegacyRawEncodable> LegacyRawEncodable for Iff<P, Q> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut s = 0;
        s += self.left.encode_raw(f);
        let r = self.right.encode_raw(f);
        s += r;
        s += encode_u64(r, f);
        f(&[legacy_magic::P_IFF]);
        s + 1
    }

    fn encoded_size(&self) -> u64 {
        self.left.encoded_size()
            + self.right.encoded_size()
            + encoded_size_u64(self.right.encoded_size())
            + 1
    }
}

impl<P: Expr, Q: Expr> Iff<P, Q> {
    pub fn subs_left<R: Expr>(self, left: R) -> Iff<R, Q> {
        Iff {
            left,
            right: self.right,
        }
    }

    pub fn subs_right<R: Expr>(self, right: R) -> Iff<P, R> {
        Iff {
            left: self.left,
            right,
        }
    }
}

define_ops_expr! { Iff<P: Expr, Q: Expr> }

/// Universal quantification over variable with a domain expression `dtype`.
pub struct ForAll<DT: Expr, P: Expr> {
    pub variable: InlineVariable,
    pub dtype: DT,
    pub inner: P,
}

impl<DT: Expr, P: Expr> expr_sealed::Sealed for ForAll<DT, P> {}
impl<DT: Expr, P: Expr> Expr for ForAll<DT, P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&DT, &P, AnyExpr>::ForAll {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
}

impl<DT: Expr + LegacyRawEncodable, P: Expr + LegacyRawEncodable> LegacyRawEncodable
    for ForAll<DT, P>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;
        size += self.dtype.encode_raw(f);
        let inner_len = self.inner.encode_raw(f);
        size += inner_len;
        size += encode_u64(inner_len, f);
        size += encode_u64(self.variable.raw(), f);
        f(&[legacy_magic::P_FORALL]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.dtype.encoded_size()
            + self.inner.encoded_size()
            + encoded_size_u64(self.inner.encoded_size())
            + encoded_size_u64(self.variable.raw())
            + 1
    }
}

impl<DT: Expr, P: Expr> ForAll<DT, P> {
    pub fn subs_inner<R: Expr>(self, inner: R) -> ForAll<DT, R> {
        ForAll {
            variable: self.variable,
            dtype: self.dtype,
            inner,
        }
    }

    pub fn subs_dtype<R: Expr>(self, dtype: R) -> ForAll<R, P> {
        ForAll {
            variable: self.variable,
            dtype,
            inner: self.inner,
        }
    }
}

define_ops_expr! { ForAll<DT: Expr, P: Expr> }

/// Existential quantification.
pub struct Exists<DT: Expr, P: Expr> {
    pub variable: InlineVariable,
    pub dtype: DT,
    pub inner: P,
}

impl<DT: Expr, P: Expr> expr_sealed::Sealed for Exists<DT, P> {}
impl<DT: Expr, P: Expr> Expr for Exists<DT, P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&DT, &P, AnyExpr>::Exists {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
}

impl<DT: Expr + LegacyRawEncodable, P: Expr + LegacyRawEncodable> LegacyRawEncodable
    for Exists<DT, P>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;
        size += self.dtype.encode_raw(f);
        let inner_len = self.inner.encode_raw(f);
        size += inner_len;
        size += encode_u64(inner_len, f);
        size += encode_u64(self.variable.raw(), f);
        f(&[legacy_magic::P_EXISTS]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.dtype.encoded_size()
            + self.inner.encoded_size()
            + encoded_size_u64(self.inner.encoded_size())
            + encoded_size_u64(self.variable.raw())
            + 1
    }
}

impl<DT: Expr, P: Expr> Exists<DT, P> {
    pub fn subs_inner<R: Expr>(self, inner: R) -> Exists<DT, R> {
        Exists {
            variable: self.variable,
            dtype: self.dtype,
            inner,
        }
    }

    pub fn subs_dtype<R: Expr>(self, dtype: R) -> Exists<R, P> {
        Exists {
            variable: self.variable,
            dtype,
            inner: self.inner,
        }
    }
}

define_ops_expr! { Exists<DT: Expr, P: Expr> }

/// Equality of two expressions.
pub struct Eq<T1: Expr, T2: Expr> {
    pub left: T1,
    pub right: T2,
}

impl<T1: Expr, T2: Expr> expr_sealed::Sealed for Eq<T1, T2> {}
impl<T1: Expr, T2: Expr> Expr for Eq<T1, T2> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&T1, &T2, AnyExpr>::Equal(&self.left, &self.right)
    }
}

impl<T1: Expr + LegacyRawEncodable, T2: Expr + LegacyRawEncodable> LegacyRawEncodable
    for Eq<T1, T2>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;
        size += self.left.encode_raw(f);
        let rlen = self.right.encode_raw(f);
        size += rlen;
        size += encode_u64(rlen, f);
        f(&[legacy_magic::P_EQUAL]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.left.encoded_size()
            + self.right.encoded_size()
            + encoded_size_u64(self.right.encoded_size())
            + 1
    }
}

impl<T1: Expr, T2: Expr> Eq<T1, T2> {
    pub fn subs_left<R: Expr>(self, left: R) -> Eq<R, T2> {
        Eq {
            left,
            right: self.right,
        }
    }

    pub fn subs_right<R: Expr>(self, right: R) -> Eq<T1, R> {
        Eq {
            left: self.left,
            right,
        }
    }
}

define_ops_expr! { Eq<T1: Expr, T2: Expr> }

// ========================= Types (as expressions) =========================

/// Boolean type.
pub struct Bool;

impl expr_sealed::Sealed for Bool {}
impl Expr for Bool {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Bool
    }
}

impl LegacyRawEncodable for Bool {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[legacy_magic::T_BOOL]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Universe of well-formed types (type of types).
pub struct Omega;

impl expr_sealed::Sealed for Omega {}
impl Expr for Omega {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Omega
    }
}

impl LegacyRawEncodable for Omega {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[legacy_magic::T_OMEGA]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Uninhabited type.
pub struct Never;

impl expr_sealed::Sealed for Never {}
impl Expr for Never {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Never
    }
}

impl LegacyRawEncodable for Never {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[legacy_magic::E_NEVER]);
        1
    }
    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Powerset type `P(A)`.
pub struct PowerSet<A: Expr> {
    pub inner: A,
}

impl<A: Expr> expr_sealed::Sealed for PowerSet<A> {}
impl<A: Expr> Expr for PowerSet<A> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, AnyExpr, AnyExpr>::Powerset(&self.inner)
    }
}

impl<A: Expr + LegacyRawEncodable> LegacyRawEncodable for PowerSet<A> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut s = 0;
        s += self.inner.encode_raw(f);
        f(&[legacy_magic::T_POWER]);
        s + 1
    }

    fn encoded_size(&self) -> u64 {
        self.inner.encoded_size() + 1
    }
}

impl<A: Expr> PowerSet<A> {
    pub fn subs_inner<R: Expr>(self, inner: R) -> PowerSet<R> {
        PowerSet { inner }
    }
}

/// Function type `A -> B`.
pub struct Func<A: Expr, B: Expr> {
    /// Domain type.
    pub domain: A,
    /// Codomain type.
    pub codomain: B,
}
impl<A: Expr, B: Expr> expr_sealed::Sealed for Func<A, B> {}
impl<A: Expr, B: Expr> Expr for Func<A, B> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, &B, AnyExpr>::Func(&self.domain, &self.codomain)
    }
}

impl<A: Expr + LegacyRawEncodable, B: Expr + LegacyRawEncodable> LegacyRawEncodable for Func<A, B> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut s = 0;
        s += self.domain.encode_raw(f);
        let r = self.codomain.encode_raw(f);
        s += r;
        s += encode_u64(r, f);
        f(&[legacy_magic::T_FUNC]);
        s + 1
    }

    fn encoded_size(&self) -> u64 {
        self.domain.encoded_size()
            + self.codomain.encoded_size()
            + encoded_size_u64(self.codomain.encoded_size())
            + 1
    }
}
