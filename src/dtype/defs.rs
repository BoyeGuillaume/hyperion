//! Concrete type constructors used to build type expressions.
//!
//! These types implement [`crate::dtype::DType`] and can be composed using methods like
//! [`crate::dtype::DType::app`], [`crate::dtype::DType::tuple`], and [`crate::dtype::DType::powerset`].
use crate::{
    dtype::{DType, dispatch::DTypeView, dtype_sealed},
    encoding::{
        RawEncodable,
        integer::{encode_u64, encoded_size_u64},
        magic,
    },
    variable::InlineVariable,
};

/// Boolean type.
pub struct TBool;

impl dtype_sealed::Sealed for TBool {}
impl DType for TBool {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        DTypeView::<Self, Self>::Bool
    }
}

impl RawEncodable for TBool {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[magic::T_BOOL]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Universe of well-formed types (the type of types).
pub struct TOmega;

impl dtype_sealed::Sealed for TOmega {}
impl DType for TOmega {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        DTypeView::<Self, Self>::Omega
    }
}

impl RawEncodable for TOmega {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[magic::T_OMEGA]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Uninhabited type (the empty type; no values inhabit it).
pub struct TNever;

impl dtype_sealed::Sealed for TNever {}
impl DType for TNever {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        DTypeView::<Self, Self>::Never
    }
}

impl RawEncodable for TNever {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&[magic::T_NEVER]);
        1
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Type variable.
impl dtype_sealed::Sealed for InlineVariable {}
impl DType for InlineVariable {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        DTypeView::<Self, Self>::Var(*self)
    }
}

// Variable RawEncodable is implemented in variable.rs and shared.

/// Function type: `A -> B`.
pub struct TApp<A: DType, B: DType> {
    /// Domain type `A`.
    pub from: A,
    /// Codomain type `B`.
    pub to: B,
}

impl<A: DType, B: DType> dtype_sealed::Sealed for TApp<A, B> {}
impl<A: DType, B: DType> DType for TApp<A, B> {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        DTypeView::<&A, &B>::Arrow(&self.from, &self.to)
    }
}

impl<A: DType + RawEncodable, B: DType + RawEncodable> RawEncodable for TApp<A, B> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.from.encode_raw(f);
        let to_len = self.to.encode_raw(f);
        size += to_len;
        size += encode_u64(to_len, f);
        f(&[magic::T_ARROW]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.from.encoded_size()
            + self.to.encoded_size()
            + encoded_size_u64(self.to.encoded_size())
            + 1
    }
}

impl<A: DType + RawEncodable, B: DType + RawEncodable> TApp<A, B> {
    pub fn subs_from<R: DType>(self, from: R) -> TApp<R, B> {
        TApp { from, to: self.to }
    }

    pub fn subs_to<L: DType>(self, to: L) -> TApp<A, L> {
        TApp {
            from: self.from,
            to,
        }
    }
}

/// Product type: `A x B`.
pub struct TTuple<A: DType, B: DType> {
    /// Left component type `A`.
    pub first: A,
    /// Right component type `B`.
    pub second: B,
}

impl<A: DType, B: DType> dtype_sealed::Sealed for TTuple<A, B> {}
impl<A: DType, B: DType> DType for TTuple<A, B> {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        DTypeView::<&A, &B>::Tuple(&self.first, &self.second)
    }
}

impl<A: DType + RawEncodable, B: DType + RawEncodable> RawEncodable for TTuple<A, B> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.first.encode_raw(f);
        let right_len = self.second.encode_raw(f);
        size += right_len;
        size += encode_u64(right_len, f);
        f(&[magic::T_TUPLE]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.first.encoded_size()
            + self.second.encoded_size()
            + encoded_size_u64(self.second.encoded_size())
            + 1
    }
}

impl<A: DType + RawEncodable, B: DType + RawEncodable> TTuple<A, B> {
    pub fn subs_first<R: DType>(self, first: R) -> TTuple<R, B> {
        TTuple {
            first,
            second: self.second,
        }
    }

    pub fn subs_second<L: DType>(self, second: L) -> TTuple<A, L> {
        TTuple {
            first: self.first,
            second,
        }
    }
}

/// Powerset type: `P(A)`.
pub struct TPowerSet<A: DType> {
    /// The element type whose powerset is taken.
    pub inner: A,
}

impl<A: DType> dtype_sealed::Sealed for TPowerSet<A> {}
impl<A: DType> DType for TPowerSet<A> {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        DTypeView::<&A, crate::dtype::DynDType>::Power(&self.inner)
    }
}

impl<A: DType + RawEncodable> RawEncodable for TPowerSet<A> {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        let mut size = 0;

        size += self.inner.encode_raw(f);
        f(&[magic::T_POWER]);
        size + 1
    }

    fn encoded_size(&self) -> u64 {
        self.inner.encoded_size() + 1
    }
}

impl<A: DType + RawEncodable> TPowerSet<A> {
    pub fn subs_inner<R: DType>(self, inner: R) -> TPowerSet<R> {
        TPowerSet { inner }
    }
}
