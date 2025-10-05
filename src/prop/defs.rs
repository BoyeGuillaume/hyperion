//! Concrete proposition constructors used to build formulas.
//!
//! These types implement [`crate::prop::Prop`] and also [`crate::expr::Expr`].
use crate::{
    dtype::{DType, DynDType},
    encoding::{
        integer::{encode_u64, encoded_size_u64},
        magic,
    },
    expr::{DynExpr, Expr, view::ExprView, expr_sealed},
    prop::{DynProp, Prop, prop_sealed},
    variable::InlineVariable,
};

use super::view::PropView;

/// Represents a true proposition.
///
/// An atomic proposition that is always true.
///
pub struct PropTrue;

impl prop_sealed::Sealed for PropTrue {}
impl expr_sealed::Sealed for PropTrue {}

impl Expr for PropTrue {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl Prop for PropTrue {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<DynProp, DynProp, DynExpr, DynExpr, DynDType>::True
    }
}

impl crate::encoding::RawEncodable for PropTrue {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[magic::P_TRUE]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

define_ops_prop! {
    PropTrue
}

/// Represents a false proposition.
///
/// An atomic proposition that is always false.
///
pub struct PropFalse;

impl prop_sealed::Sealed for PropFalse {}
impl expr_sealed::Sealed for PropFalse {}

impl Expr for PropFalse {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl Prop for PropFalse {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<DynProp, DynProp, DynExpr, DynExpr, DynDType>::False
    }
}

impl crate::encoding::RawEncodable for PropFalse {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[magic::P_FALSE]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

define_ops_prop! {
    PropFalse
}

/// Represents the negation of a proposition.
///
/// If `P` is a proposition, then `Not<P>` represents the proposition "not P".
///
pub struct Not<P: Prop> {
    /// Inner proposition being negated.
    pub inner: P,
}

impl<P: Prop> prop_sealed::Sealed for Not<P> {}
impl<P: Prop> expr_sealed::Sealed for Not<P> {}

impl<P: Prop> Expr for Not<P> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop> Prop for Not<P> {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<&P, DynProp, DynExpr, DynExpr, DynDType>::Not(&self.inner)
    }
}

impl<P: Prop + crate::encoding::RawEncodable> crate::encoding::RawEncodable for Not<P> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let size = self.inner.encode_raw(f);
        f(&[magic::P_NOT]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.inner.encoded_size() + 1
    }
}

impl<P: Prop> Not<P> {
    /// Substitute the inner proposition while keeping the same negation.
    pub fn subs_inner<R: Prop>(self, inner: R) -> Not<R> {
        Not { inner }
    }
}

define_ops_prop! {
    Not<P: Prop>
}

/// Represents the conjunction (logical AND) of two propositions.
///
/// If `P` and `Q` are propositions, then `And<P, Q>` represents the proposition "P and Q".
///
/// This struct holds two fields, `left` and `right`, which are the two propositions being conjoined.
pub struct And<P: Prop, Q: Prop> {
    /// Left-hand proposition.
    pub left: P,
    /// Right-hand proposition.
    pub right: Q,
}

impl<P: Prop, Q: Prop> prop_sealed::Sealed for And<P, Q> {}
impl<P: Prop, Q: Prop> expr_sealed::Sealed for And<P, Q> {}

impl<P: Prop, Q: Prop> Expr for And<P, Q> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for And<P, Q> {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<&P, &Q, DynExpr, DynExpr, DynDType>::And(&self.left, &self.right)
    }
}

impl<P: Prop + crate::encoding::RawEncodable, Q: Prop + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for And<P, Q>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.left.encode_raw(f);

        let right_len = self.right.encode_raw(f);
        size += right_len;

        size += encode_u64(right_len, f);
        f(&[magic::P_AND]);

        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.left.encoded_size()
            + self.right.encoded_size()
            + encoded_size_u64(self.right.encoded_size())
            + 1
    }
}

impl<P: Prop, Q: Prop> And<P, Q> {
    /// Substitute the left-hand proposition while keeping the same right-hand proposition.
    pub fn subs_left<R: Prop>(self, left: R) -> And<R, Q> {
        And {
            left,
            right: self.right,
        }
    }

    /// Substitute the right-hand proposition while keeping the same left-hand proposition.
    pub fn subs_right<R: Prop>(self, right: R) -> And<P, R> {
        And {
            left: self.left,
            right,
        }
    }
}

define_ops_prop! {
    And<P: Prop, Q: Prop>
}

/// Represents the disjunction (logical OR) of two propositions.
///
/// If `P` and `Q` are propositions, then `Or<P, Q>` represents the proposition "P or Q".
///
/// This struct holds two fields, `left` and `right`, which are the two propositions being disjoined.
pub struct Or<P: Prop, Q: Prop> {
    /// Left-hand proposition.
    pub left: P,
    /// Right-hand proposition.
    pub right: Q,
}

impl<P: Prop, Q: Prop> prop_sealed::Sealed for Or<P, Q> {}
impl<P: Prop, Q: Prop> expr_sealed::Sealed for Or<P, Q> {}

impl<P: Prop, Q: Prop> Expr for Or<P, Q> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Or<P, Q> {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<&P, &Q, DynExpr, DynExpr, DynDType>::Or(&self.left, &self.right)
    }
}

impl<P: Prop + crate::encoding::RawEncodable, Q: Prop + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for Or<P, Q>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.left.encode_raw(f);

        let right_len = self.right.encode_raw(f);
        size += right_len;

        size += encode_u64(right_len, f);
        f(&[magic::P_OR]);

        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.left.encoded_size()
            + self.right.encoded_size()
            + encoded_size_u64(self.right.encoded_size())
            + 1
    }
}

impl<P: Prop, Q: Prop> Or<P, Q> {
    /// Substitute the left-hand proposition while keeping the same right-hand proposition.
    pub fn subs_left<R: Prop>(self, left: R) -> Or<R, Q> {
        Or {
            left,
            right: self.right,
        }
    }

    /// Substitute the right-hand proposition while keeping the same left-hand proposition.
    pub fn subs_right<R: Prop>(self, right: R) -> Or<P, R> {
        Or {
            left: self.left,
            right,
        }
    }
}

define_ops_prop! {
    Or<P: Prop, Q: Prop>
}

/// Represents the implication (logical IF-THEN) between two propositions.
///
/// If `P` and `Q` are propositions, then `Imp<P, Q>` represents the proposition "if P then Q".
/// This struct holds two fields, `antecedent` and `consequent`, which are the
/// propositions involved in the implication.
pub struct Imp<P: Prop, Q: Prop> {
    /// Antecedent (premise).
    pub antecedent: P,
    /// Consequent (conclusion).
    pub consequent: Q,
}

impl<P: Prop, Q: Prop> prop_sealed::Sealed for Imp<P, Q> {}
impl<P: Prop, Q: Prop> expr_sealed::Sealed for Imp<P, Q> {}

impl<P: Prop, Q: Prop> Expr for Imp<P, Q> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Imp<P, Q> {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<&P, &Q, DynExpr, DynExpr, DynDType>::Implies(&self.antecedent, &self.consequent)
    }
}

impl<P: Prop + crate::encoding::RawEncodable, Q: Prop + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for Imp<P, Q>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.antecedent.encode_raw(f);
        let right_len = self.consequent.encode_raw(f);
        size += right_len;
        size += encode_u64(right_len, f);
        f(&[magic::P_IMPLIES]);

        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.antecedent.encoded_size()
            + self.consequent.encoded_size()
            + encoded_size_u64(self.consequent.encoded_size())
            + 1
    }
}

impl<P: Prop, Q: Prop> Imp<P, Q> {
    /// Substitute the antecedent while keeping the same consequent.
    pub fn subs_antecedent<R: Prop>(self, antecedent: R) -> Imp<R, Q> {
        Imp {
            antecedent,
            consequent: self.consequent,
        }
    }

    /// Substitute the consequent while keeping the same antecedent.
    pub fn subs_consequent<R: Prop>(self, consequent: R) -> Imp<P, R> {
        Imp {
            antecedent: self.antecedent,
            consequent,
        }
    }
}

define_ops_prop! {
    Imp<P: Prop, Q: Prop>
}

/// Represents the biconditional (logical IF AND ONLY IF) between two propositions.
///
/// If `P` and `Q` are propositions, then `Iff<P, Q>` represents the proposition "P if and only if Q".
/// This struct holds two fields, `left` and `right`, which are the propositions involved
/// in the biconditional.
pub struct Iff<P: Prop, Q: Prop> {
    /// Left-hand proposition.
    pub left: P,
    /// Right-hand proposition.
    pub right: Q,
}

impl<P: Prop, Q: Prop> prop_sealed::Sealed for Iff<P, Q> {}
impl<P: Prop, Q: Prop> expr_sealed::Sealed for Iff<P, Q> {}

impl<P: Prop, Q: Prop> Expr for Iff<P, Q> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Iff<P, Q> {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<&P, &Q, DynExpr, DynExpr, DynDType>::Iff(&self.left, &self.right)
    }
}

impl<P: Prop + crate::encoding::RawEncodable, Q: Prop + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for Iff<P, Q>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.left.encode_raw(f);
        let right_len = self.right.encode_raw(f);

        size += right_len;
        size += encode_u64(right_len, f);
        f(&[magic::P_IFF]);

        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.left.encoded_size()
            + self.right.encoded_size()
            + encoded_size_u64(self.right.encoded_size())
            + 1
    }
}

impl<P: Prop, Q: Prop> Iff<P, Q> {
    /// Substitute the left-hand proposition while keeping the same right-hand proposition.
    pub fn subs_left<R: Prop>(self, left: R) -> Iff<R, Q> {
        Iff {
            left,
            right: self.right,
        }
    }

    /// Substitute the right-hand proposition while keeping the same left-hand proposition.
    pub fn subs_right<R: Prop>(self, right: R) -> Iff<P, R> {
        Iff {
            left: self.left,
            right,
        }
    }
}

define_ops_prop! {
    Iff<P: Prop, Q: Prop>
}

/// Represents a universally quantified proposition.
///
/// If `P` is a proposition and `DT` is a type, then `ForAll<DT, P>` represents the proposition
/// "for all x of type DT, P(x)".
pub struct ForAll<DT: DType, P: Prop> {
    /// Bound variable identifier.
    pub variable: InlineVariable,
    /// Domain type of the bound variable.
    pub dtype: DT,
    /// Inner proposition.
    pub inner: P,
}

impl<DT: DType, P: Prop> prop_sealed::Sealed for ForAll<DT, P> {}
impl<DT: DType, P: Prop> expr_sealed::Sealed for ForAll<DT, P> {}

impl<DT: DType, P: Prop> Expr for ForAll<DT, P> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<DT: DType, P: Prop> Prop for ForAll<DT, P> {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<&P, DynProp, DynExpr, DynExpr, &DT>::ForAll {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
}

impl<DT, P> crate::encoding::RawEncodable for ForAll<DT, P>
where
    DT: DType + crate::encoding::RawEncodable,
    P: Prop + crate::encoding::RawEncodable,
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.dtype.encode_raw(f);
        let inner_len = self.inner.encode_raw(f);
        size += inner_len;
        size += encode_u64(inner_len, f);
        size += encode_u64(self.variable.raw(), f);
        f(&[magic::P_FORALL]);

        size + 1
    }

    // fn encode_raw(&self, buf: &mut DynBuf) {
    //     self.dtype.encode_raw(buf);
    //     let inner_start = buf.len();
    //     self.inner.encode_raw(buf);
    //     let inner_len = buf.len() - inner_start;
    //     push_len(inner_len, buf);
    //     crate::encoding::integer::encode_u64(self.variable.raw(), buf);
    //     buf.push(magic::P_FORALL);
    // }

    fn encoded_size(&self) -> u64 {
        self.dtype.encoded_size()
            + self.inner.encoded_size()
            + encoded_size_u64(self.inner.encoded_size())
            + encoded_size_u64(self.variable.raw())
            + 1
    }
}

impl<DT: DType, P: Prop> ForAll<DT, P> {
    /// Substitute the inner proposition while keeping the same quantifier.
    pub fn subs_inner<R: Prop>(self, inner: R) -> ForAll<DT, R> {
        ForAll {
            variable: self.variable,
            dtype: self.dtype,
            inner,
        }
    }

    /// Substitute the domain type while keeping the same inner proposition.
    pub fn subs_dtype<R: DType>(self, dtype: R) -> ForAll<R, P> {
        ForAll {
            variable: self.variable,
            dtype,
            inner: self.inner,
        }
    }
}

define_ops_prop! {
    ForAll<DT: DType, P: Prop>
}

/// Represents an existentially quantified proposition.
///
/// If `P` is a proposition and `DT` is a type, then `Exists<DT, P>` represents the proposition
/// "there exists an x of type DT such that P(x)".
pub struct Exists<DT: DType, P: Prop> {
    /// Bound variable identifier.
    pub variable: InlineVariable,
    /// Domain type of the bound variable.
    pub dtype: DT,
    /// Inner proposition.
    pub inner: P,
}

impl<DT: DType, P: Prop> prop_sealed::Sealed for Exists<DT, P> {}
impl<DT: DType, P: Prop> expr_sealed::Sealed for Exists<DT, P> {}

impl<DT: DType, P: Prop> Expr for Exists<DT, P> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<DT: DType, P: Prop> Prop for Exists<DT, P> {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<&P, DynProp, DynExpr, DynExpr, &DT>::Exists {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
}

impl<DT, P> crate::encoding::RawEncodable for Exists<DT, P>
where
    DT: DType + crate::encoding::RawEncodable,
    P: Prop + crate::encoding::RawEncodable,
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.dtype.encode_raw(f);
        let inner_len = self.inner.encode_raw(f);
        size += inner_len;
        size += encode_u64(inner_len, f);
        size += encode_u64(self.variable.raw(), f);
        f(&[magic::P_EXISTS]);

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

impl<DT: DType, P: Prop> Exists<DT, P> {
    /// Substitute the inner proposition while keeping the same quantifier.
    pub fn subs_inner<R: Prop>(self, inner: R) -> Exists<DT, R> {
        Exists {
            variable: self.variable,
            dtype: self.dtype,
            inner,
        }
    }

    /// Substitute the domain type while keeping the same inner proposition.
    pub fn subs_dtype<R: DType>(self, dtype: R) -> Exists<R, P> {
        Exists {
            variable: self.variable,
            dtype,
            inner: self.inner,
        }
    }
}

define_ops_prop! {
    Exists<DT: DType, P: Prop>
}

/// Represents the equality of two exprs.
///
/// If `T1` and `T2` are two exprs, then `Eq<T1, T2>` represents the proposition "T1 is equal to T2".
/// This struct holds two fields, `left` and `right`, which are the exprs being compared for equality.
pub struct Eq<T1: Expr, T2: Expr> {
    /// Left expression.
    pub left: T1,
    /// Right expression.
    pub right: T2,
}

impl<T1: Expr, T2: Expr> prop_sealed::Sealed for Eq<T1, T2> {}
impl<T1: Expr, T2: Expr> expr_sealed::Sealed for Eq<T1, T2> {}

impl<T1: Expr, T2: Expr> Expr for Eq<T1, T2> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        ExprView::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<T1: Expr, T2: Expr> Prop for Eq<T1, T2> {
    fn view_prop(
        &self,
    ) -> PropView<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropView::<DynProp, DynProp, &T1, &T2, DynDType>::Equal(&self.left, &self.right)
    }
}

impl<T1: Expr + crate::encoding::RawEncodable, T2: Expr + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for Eq<T1, T2>
{
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.left.encode_raw(f);
        let right_len = self.right.encode_raw(f);

        size += right_len;
        size += encode_u64(right_len, f);
        f(&[magic::P_EQUAL]);

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
    /// Substitute the left expression while keeping the same right expression.
    pub fn subs_left<R: Expr>(self, left: R) -> Eq<R, T2> {
        Eq {
            left,
            right: self.right,
        }
    }

    /// Substitute the right expression while keeping the same left expression.
    pub fn subs_right<R: Expr>(self, right: R) -> Eq<T1, R> {
        Eq {
            left: self.left,
            right,
        }
    }
}

define_ops_prop! {
    Eq<T1: Expr, T2: Expr>
}
