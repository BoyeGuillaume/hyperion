use crate::{
    dtype::{DType, dispatch::DTypeDispatch, dtype_sealed},
    variable::InlineVariable,
};

/// Boolean type
pub struct TBool;

impl dtype_sealed::Sealed for TBool {}
impl DType for TBool {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<Self, Self>::Bool
    }
}

/// Universe of well-formed types (aka Type or Set depending on reading)
pub struct TOmega;

impl dtype_sealed::Sealed for TOmega {}
impl DType for TOmega {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<Self, Self>::Omega
    }
}

/// Uninhabited type
pub struct TNever;

impl dtype_sealed::Sealed for TNever {}
impl DType for TNever {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<Self, Self>::Never
    }
}

/// Type variable
impl dtype_sealed::Sealed for InlineVariable {}
impl DType for InlineVariable {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<Self, Self>::Var(*self)
    }
}

/// Function type: A -> B
pub struct TFun<A: DType, B: DType> {
    pub from: A,
    pub to: B,
}

impl<A: DType, B: DType> dtype_sealed::Sealed for TFun<A, B> {}
impl<A: DType, B: DType> DType for TFun<A, B> {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<&A, &B>::Arrow(&self.from, &self.to)
    }
}

/// Product type: A x B
pub struct TTuple<A: DType, B: DType> {
    pub first: A,
    pub second: B,
}

impl<A: DType, B: DType> dtype_sealed::Sealed for TTuple<A, B> {}
impl<A: DType, B: DType> DType for TTuple<A, B> {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<&A, &B>::Tuple(&self.first, &self.second)
    }
}

/// Powerset type: P(A)
pub struct TPowerSet<A: DType> {
    pub inner: A,
}

impl<A: DType> dtype_sealed::Sealed for TPowerSet<A> {}
impl<A: DType> DType for TPowerSet<A> {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<&A, crate::dtype::DynDType>::Power(&self.inner)
    }
}
