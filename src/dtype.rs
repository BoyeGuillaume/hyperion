pub mod defs;
pub mod dispatch;
pub use defs::*;

pub(crate) mod dtype_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait DType: dtype_sealed::Sealed + Sized {
    fn decode_dtype(
        &self,
    ) -> crate::dtype::dispatch::DTypeDispatch<impl crate::dtype::DType, impl crate::dtype::DType>;
}

impl<'a, T: DType> DType for &'a T {
    fn decode_dtype(
        &self,
    ) -> crate::dtype::dispatch::DTypeDispatch<impl crate::dtype::DType, impl crate::dtype::DType>
    {
        (*self).decode_dtype()
    }
}

pub struct DynDType {}
impl dtype_sealed::Sealed for DynDType {}
impl DType for DynDType {
    fn decode_dtype(
        &self,
    ) -> crate::dtype::dispatch::DTypeDispatch<impl crate::dtype::DType, impl crate::dtype::DType>
    {
        crate::dtype::dispatch::DTypeDispatch::<crate::dtype::DynDType, crate::dtype::DynDType>::Omega
    }
}
