pub mod defs;
pub mod dispatch;

pub(crate) mod term_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait Term: term_sealed::Sealed + Sized {}

impl<'a, T: Term> Term for &'a T {}

pub struct DynTerm {}
impl term_sealed::Sealed for DynTerm {}
impl Term for DynTerm {}
