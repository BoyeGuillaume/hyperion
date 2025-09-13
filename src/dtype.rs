pub mod defs;
pub mod dispatch;
pub use defs::*;

use crate::dtype::dispatch::DTypeDispatch;

pub(crate) mod dtype_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait DType: dtype_sealed::Sealed + Sized {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType>;
}

impl<'a, T: DType> DType for &'a T {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        (*self).decode_dtype()
    }
}

pub struct DynDType {}
impl dtype_sealed::Sealed for DynDType {}

impl DType for DynDType {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<crate::dtype::DynDType, crate::dtype::DynDType>::Omega
    }
}

impl crate::encoding::RawEncodable for DynDType {
    fn encode_raw(&self, buf: &mut crate::encoding::DynBuf) {
        // By convention, DynDType encodes to Omega unless specified otherwise by a concrete type.
        buf.push(crate::encoding::magic::T_OMEGA);
    }
}
