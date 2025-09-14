use crate::{
    dtype::{DType, DynDType},
    encoding::{DynBuf, magic, push_len},
    expr::{DynExpr, Expr, dispatch::ExprDispatch, expr_sealed},
    prop::{DynProp, Prop, prop_sealed},
    variable::InlineVariable,
};

use super::dispatch::PropDispatch;

/// Represents a true proposition.
///
/// An atomic proposition that is always true.
///
pub struct PropTrue;

impl prop_sealed::Sealed for PropTrue {}
impl expr_sealed::Sealed for PropTrue {}

impl Expr for PropTrue {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl Prop for PropTrue {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<DynProp, DynProp, DynExpr, DynExpr, DynDType>::True
    }
}

impl crate::encoding::RawEncodable for PropTrue {
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.push(magic::P_TRUE);
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
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl Prop for PropFalse {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<DynProp, DynProp, DynExpr, DynExpr, DynDType>::False
    }
}

impl crate::encoding::RawEncodable for PropFalse {
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.push(magic::P_FALSE);
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
    pub inner: P,
}

impl<P: Prop> prop_sealed::Sealed for Not<P> {}
impl<P: Prop> expr_sealed::Sealed for Not<P> {}

impl<P: Prop> Expr for Not<P> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop> Prop for Not<P> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, DynProp, DynExpr, DynExpr, DynDType>::Not(&self.inner)
    }
}

impl<P: Prop + crate::encoding::RawEncodable> crate::encoding::RawEncodable for Not<P> {
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.inner.encode_raw(buf);
        buf.push(magic::P_NOT);
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
    pub left: P,
    pub right: Q,
}

impl<P: Prop, Q: Prop> prop_sealed::Sealed for And<P, Q> {}
impl<P: Prop, Q: Prop> expr_sealed::Sealed for And<P, Q> {}

impl<P: Prop, Q: Prop> Expr for And<P, Q> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for And<P, Q> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, &Q, DynExpr, DynExpr, DynDType>::And(&self.left, &self.right)
    }
}

impl<P: Prop + crate::encoding::RawEncodable, Q: Prop + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for And<P, Q>
{
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.left.encode_raw(buf);
        let right_start = buf.len();
        self.right.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::P_AND);
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
    pub left: P,
    pub right: Q,
}

impl<P: Prop, Q: Prop> prop_sealed::Sealed for Or<P, Q> {}
impl<P: Prop, Q: Prop> expr_sealed::Sealed for Or<P, Q> {}

impl<P: Prop, Q: Prop> Expr for Or<P, Q> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Or<P, Q> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, &Q, DynExpr, DynExpr, DynDType>::Or(&self.left, &self.right)
    }
}

impl<P: Prop + crate::encoding::RawEncodable, Q: Prop + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for Or<P, Q>
{
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.left.encode_raw(buf);
        let right_start = buf.len();
        self.right.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::P_OR);
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
    pub antecedent: P,
    pub consequent: Q,
}

impl<P: Prop, Q: Prop> prop_sealed::Sealed for Imp<P, Q> {}
impl<P: Prop, Q: Prop> expr_sealed::Sealed for Imp<P, Q> {}

impl<P: Prop, Q: Prop> Expr for Imp<P, Q> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Imp<P, Q> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, &Q, DynExpr, DynExpr, DynDType>::Implies(
            &self.antecedent,
            &self.consequent,
        )
    }
}

impl<P: Prop + crate::encoding::RawEncodable, Q: Prop + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for Imp<P, Q>
{
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.antecedent.encode_raw(buf);
        let right_start = buf.len();
        self.consequent.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::P_IMPLIES);
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
    pub left: P,
    pub right: Q,
}

impl<P: Prop, Q: Prop> prop_sealed::Sealed for Iff<P, Q> {}
impl<P: Prop, Q: Prop> expr_sealed::Sealed for Iff<P, Q> {}

impl<P: Prop, Q: Prop> Expr for Iff<P, Q> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Iff<P, Q> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, &Q, DynExpr, DynExpr, DynDType>::Iff(&self.left, &self.right)
    }
}

impl<P: Prop + crate::encoding::RawEncodable, Q: Prop + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for Iff<P, Q>
{
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.left.encode_raw(buf);
        let right_start = buf.len();
        self.right.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::P_IFF);
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
    pub variable: InlineVariable,
    pub dtype: DT,
    pub inner: P,
}

impl<DT: DType, P: Prop> prop_sealed::Sealed for ForAll<DT, P> {}
impl<DT: DType, P: Prop> expr_sealed::Sealed for ForAll<DT, P> {}

impl<DT: DType, P: Prop> Expr for ForAll<DT, P> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<DT: DType, P: Prop> Prop for ForAll<DT, P> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, DynProp, DynExpr, DynExpr, &DT>::ForAll {
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
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.dtype.encode_raw(buf);
        let inner_start = buf.len();
        self.inner.encode_raw(buf);
        let inner_len = buf.len() - inner_start;
        push_len(inner_len, buf);
        crate::encoding::integer::encode_u64(self.variable.id(), buf);
        buf.push(magic::P_FORALL);
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
    pub variable: InlineVariable,
    pub dtype: DT,
    pub inner: P,
}

impl<DT: DType, P: Prop> prop_sealed::Sealed for Exists<DT, P> {}
impl<DT: DType, P: Prop> expr_sealed::Sealed for Exists<DT, P> {}

impl<DT: DType, P: Prop> Expr for Exists<DT, P> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<DT: DType, P: Prop> Prop for Exists<DT, P> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, DynProp, DynExpr, DynExpr, &DT>::Exists {
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
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.dtype.encode_raw(buf);
        let inner_start = buf.len();
        self.inner.encode_raw(buf);
        let inner_len = buf.len() - inner_start;
        push_len(inner_len, buf);
        crate::encoding::integer::encode_u64(self.variable.id(), buf);
        buf.push(magic::P_EXISTS);
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
    pub left: T1,
    pub right: T2,
}

impl<T1: Expr, T2: Expr> prop_sealed::Sealed for Eq<T1, T2> {}
impl<T1: Expr, T2: Expr> expr_sealed::Sealed for Eq<T1, T2> {}

impl<T1: Expr, T2: Expr> Expr for Eq<T1, T2> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<T1: Expr, T2: Expr> Prop for Eq<T1, T2> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<DynProp, DynProp, &T1, &T2, DynDType>::Equal(&self.left, &self.right)
    }
}

impl<T1: Expr + crate::encoding::RawEncodable, T2: Expr + crate::encoding::RawEncodable>
    crate::encoding::RawEncodable for Eq<T1, T2>
{
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.left.encode_raw(buf);
        let right_start = buf.len();
        self.right.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::P_EQUAL);
    }
}

define_ops_prop! {
    Eq<T1: Expr, T2: Expr>
}
