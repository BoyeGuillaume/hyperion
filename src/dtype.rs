pub(crate) mod dtype_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait DType: dtype_sealed::Sealed + Sized {}

impl<'a, T: DType> DType for &'a T {}

pub struct DynDType {}
impl dtype_sealed::Sealed for DynDType {}
impl DType for DynDType {}
