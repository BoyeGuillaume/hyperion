use crate::{
    dtype::DynDType,
    prop::dispatch::PropDispatch,
    term::{DynTerm, Term},
};

mod defs;
pub mod dispatch;
pub use defs::*;

pub(crate) mod prop_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait Prop: prop_sealed::Sealed + Sized {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Term, impl Term, impl crate::dtype::DType>;

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
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Term, impl Term, impl crate::dtype::DType> {
        (*self).decode()
    }
}

pub struct DynProp {}

impl prop_sealed::Sealed for DynProp {}
impl Prop for DynProp {
    fn decode(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Term, impl Term, impl crate::dtype::DType> {
        PropDispatch::<DynProp, DynProp, DynTerm, DynTerm, DynDType>::True
    }
}
