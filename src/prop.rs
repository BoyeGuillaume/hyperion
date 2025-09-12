pub mod defs;
pub mod dispatch;

pub(crate) mod prop_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait Prop: prop_sealed::Sealed + Sized {}
