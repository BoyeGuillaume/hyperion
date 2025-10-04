//! Concrete type constructors used to build type expressions.
//!
//! These types implement [`crate::dtype::DType`] and can be composed using methods like
//! [`crate::dtype::DType::app`], [`crate::dtype::DType::tuple`], and [`crate::dtype::DType::powerset`].
use crate::{
    dtype::{DType, dispatch::DTypeDispatch, dtype_sealed},
    encoding::{DynBuf, RawEncodable, integer::encoded_size_u64, magic, push_len},
    variable::InlineVariable,
};

/// Boolean type.
pub struct TBool;

impl dtype_sealed::Sealed for TBool {}
impl DType for TBool {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<Self, Self>::Bool
    }
}

impl RawEncodable for TBool {
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.push(magic::T_BOOL);
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Universe of well-formed types (the type of types).
pub struct TOmega;

impl dtype_sealed::Sealed for TOmega {}
impl DType for TOmega {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<Self, Self>::Omega
    }
}

impl RawEncodable for TOmega {
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.push(magic::T_OMEGA);
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Uninhabited type (the empty type; no values inhabit it).
pub struct TNever;

impl dtype_sealed::Sealed for TNever {}
impl DType for TNever {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<Self, Self>::Never
    }
}

impl RawEncodable for TNever {
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.push(magic::T_NEVER);
    }

    fn encoded_size(&self) -> u64 {
        1
    }
}

/// Type variable.
impl dtype_sealed::Sealed for InlineVariable {}
impl DType for InlineVariable {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<Self, Self>::Var(*self)
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
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<&A, &B>::Arrow(&self.from, &self.to)
    }
}

impl<A: DType + RawEncodable, B: DType + RawEncodable> RawEncodable for TApp<A, B> {
    fn encode_raw(&self, buf: &mut DynBuf) {
        let start = buf.len();
        self.from.encode_raw(buf);
        let right_start = buf.len();
        self.to.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::T_ARROW);
        debug_assert!(buf.len() >= start);
    }

    fn encoded_size(&self) -> u64 {
        self.from.encoded_size()
            + self.to.encoded_size()
            + encoded_size_u64(self.to.encoded_size())
            + 1
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
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<&A, &B>::Tuple(&self.first, &self.second)
    }
}

impl<A: DType + RawEncodable, B: DType + RawEncodable> RawEncodable for TTuple<A, B> {
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.first.encode_raw(buf);
        let right_start = buf.len();
        self.second.encode_raw(buf);
        let right_len = buf.len() - right_start;
        push_len(right_len, buf);
        buf.push(magic::T_TUPLE);
    }

    fn encoded_size(&self) -> u64 {
        self.first.encoded_size()
            + self.second.encoded_size()
            + encoded_size_u64(self.second.encoded_size())
            + 1
    }
}

/// Powerset type: `P(A)`.
pub struct TPowerSet<A: DType> {
    /// The element type whose powerset is taken.
    pub inner: A,
}

impl<A: DType> dtype_sealed::Sealed for TPowerSet<A> {}
impl<A: DType> DType for TPowerSet<A> {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DTypeDispatch::<&A, crate::dtype::DynDType>::Power(&self.inner)
    }
}

impl<A: DType + RawEncodable> RawEncodable for TPowerSet<A> {
    fn encode_raw(&self, buf: &mut DynBuf) {
        self.inner.encode_raw(buf);
        buf.push(magic::T_POWER);
    }

    fn encoded_size(&self) -> u64 {
        self.inner.encoded_size() + 1
    }
}
