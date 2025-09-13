use crate::{
    dtype::{DType, DynDType},
    expr::{DynExpr, Expr, dispatch::ExprDispatch, expr_sealed},
    prop::{DynProp, Prop, prop_sealed},
    variable::InlineVariable,
};

use super::dispatch::PropDispatch;

/// Represents a true proposition.
///
/// An atomic proposition that is always true.
///
pub struct PropTrue {}

impl prop_sealed::Sealed for PropTrue {}
impl expr_sealed::Sealed for PropTrue {}

impl Expr for PropTrue {
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl Prop for PropTrue {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<DynProp, DynProp, DynExpr, DynExpr, DynDType>::True
    }
}

/// Represents a false proposition.
///
/// An atomic proposition that is always false.
///
pub struct PropFalse {}

impl prop_sealed::Sealed for PropFalse {}
impl expr_sealed::Sealed for PropFalse {}

impl Expr for PropFalse {
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl Prop for PropFalse {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<DynProp, DynProp, DynExpr, DynExpr, DynDType>::False
    }
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
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop> Prop for Not<P> {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, DynProp, DynExpr, DynExpr, DynDType>::Not(&self.inner)
    }
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
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for And<P, Q> {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, &Q, DynExpr, DynExpr, DynDType>::And(&self.left, &self.right)
    }
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
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Or<P, Q> {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, &Q, DynExpr, DynExpr, DynDType>::Or(&self.left, &self.right)
    }
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
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Imp<P, Q> {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, &Q, DynExpr, DynExpr, DynDType>::Implies(
            &self.antecedent,
            &self.consequent,
        )
    }
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
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<P: Prop, Q: Prop> Prop for Iff<P, Q> {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, &Q, DynExpr, DynExpr, DynDType>::Iff(&self.left, &self.right)
    }
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
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<DT: DType, P: Prop> Prop for ForAll<DT, P> {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, DynProp, DynExpr, DynExpr, &DT>::ForAll {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
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
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<DT: DType, P: Prop> Prop for Exists<DT, P> {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<&P, DynProp, DynExpr, DynExpr, &DT>::Exists {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
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
    fn dispatch(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl<T1: Expr, T2: Expr> Prop for Eq<T1, T2> {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<DynProp, DynProp, &T1, &T2, DynDType>::Equal(&self.left, &self.right)
    }
}
