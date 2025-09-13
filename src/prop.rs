use crate::{
    dtype::DynDType,
    expr::{DynExpr, Expr, dispatch::ExprDispatch, expr_sealed},
    prop::dispatch::PropDispatch,
};

mod defs;
pub mod dispatch;
pub use defs::*;

pub(crate) mod prop_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait Prop: Expr + prop_sealed::Sealed + Sized {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType>;

    fn and<Q: Prop>(self, other: Q) -> And<Self, Q>
    where
        Self: Sized,
    {
        And {
            left: self,
            right: other,
        }
    }

    fn or<Q: Prop>(self, other: Q) -> Or<Self, Q>
    where
        Self: Sized,
    {
        Or {
            left: self,
            right: other,
        }
    }

    fn implies<Q: Prop>(self, other: Q) -> Imp<Self, Q>
    where
        Self: Sized,
    {
        Imp {
            antecedent: self,
            consequent: other,
        }
    }

    fn iff<Q: Prop>(self, other: Q) -> Iff<Self, Q>
    where
        Self: Sized,
    {
        Iff {
            left: self,
            right: other,
        }
    }

    fn not(self) -> Not<Self>
    where
        Self: Sized,
    {
        Not { inner: self }
    }
}

impl<'a, T: Prop> Prop for &'a T {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        (*self).decode_prop()
    }
}

pub struct DynProp {}

impl prop_sealed::Sealed for DynProp {}
impl expr_sealed::Sealed for DynProp {}

impl Expr for DynProp {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl Prop for DynProp {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        PropDispatch::<DynProp, DynProp, DynExpr, DynExpr, DynDType>::True
    }
}

impl crate::encoding::RawEncodable for DynProp {
    fn encode_raw(&self, buf: &mut crate::encoding::DynBuf) {
        // By convention, encode as True if the dynamic value has no additional info.
        buf.push(crate::encoding::magic::P_TRUE);
    }
}
