//! Type constructors and zero-copy dynamic types.
//!
//! This module exposes:
//! - concrete type constructors like [`defs::TBool`], [`defs::TTuple`], [`defs::TApp`], …
//! - the [`DType`] trait implemented by all type expressions
//! - dynamic encodings via [`DynDType`] and [`DynBorrowedDType`]
//!
//! Types can be composed with ergonomic helpers on [`DType`], then encoded to a compact
//! byte buffer and decoded later without allocation.
pub mod defs;
pub mod view;
pub use defs::*;

use crate::dtype::view::DTypeView;
use crate::encoding::{DynBuf, RawEncodable};
use crate::{encoding, variable::InlineVariable};

pub(crate) mod dtype_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

/// Trait implemented by all type expressions provided by this crate.
///
/// The main entry points are:
/// - [`encode`](Self::encode): produce a compact dynamic representation
/// - [`decode_dtype`](Self::decode_dtype): describe the top-level shape
/// - helpers like [`app`](Self::app), [`tuple`](Self::tuple), and [`powerset`](Self::powerset)
pub trait DType: dtype_sealed::Sealed + Sized + RawEncodable {
    /// Describe the type's outer constructor and expose its children in a dispatch enum.
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType>;

    /// Encode this type into a dynamic, byte-backed representation.
    #[inline]
    fn encode(&self) -> DynDType {
        let mut buf = DynBuf::new();
        self.encode_dynbuf(&mut buf);
        DynDType { bytes: buf }
    }

    /// Construct a function type from `self` to `arg`.
    ///
    /// Equivalent to [`Self::app`].
    #[inline]
    fn application<Q: DType>(self, arg: Q) -> defs::TApp<Self, Q> {
        defs::TApp {
            from: self,
            to: arg,
        }
    }

    /// Construct a function type from `self` to `arg`.
    #[inline]
    fn app<Q: DType>(self, arg: Q) -> defs::TApp<Self, Q> {
        self.application(arg)
    }

    /// Construct a binary product type `self x other`.
    #[inline]
    fn tuple<Q: DType>(self, other: Q) -> defs::TTuple<Self, Q>
    where
        Self: Sized,
    {
        defs::TTuple {
            first: self,
            second: other,
        }
    }

    /// Construct the powerset type `P(self)`.
    #[inline]
    fn powerset(self) -> defs::TPowerSet<Self>
    where
        Self: Sized,
    {
        defs::TPowerSet { inner: self }
    }
}

impl<'a, T: DType> DType for &'a T {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        (*self).view_dtype()
    }
}

/// Dynamically-encoded DType backed by a compact byte buffer.
///
/// Stores up to 32 bytes inline before spilling to the heap.
pub struct DynDType {
    pub(crate) bytes: DynBuf,
}
impl dtype_sealed::Sealed for DynDType {}

impl DType for DynDType {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        DynBorrowedDType::raw_decode_dtype(&self.bytes)
    }
}

impl RawEncodable for DynDType {
    #[inline]
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&self.bytes);
        self.bytes.len() as u64
    }

    fn encoded_size(&self) -> u64 {
        self.bytes.len() as u64
    }
}

impl DynDType {
    /// Borrow these bytes as a zero-copy dynamic type.
    #[inline]
    pub fn as_borrowed(&self) -> DynBorrowedDType<'_> {
        DynBorrowedDType {
            bytes: self.bytes.as_slice(),
        }
    }

    /// Zero-copy decode returning borrowed children.
    ///
    /// This avoids allocations and returns borrowed subtypes when traversing.
    pub fn decode_dtype_concrete(&self) -> DTypeView<DynBorrowedDType<'_>, DynBorrowedDType<'_>> {
        self.as_borrowed().decode_dtype_concrete()
    }
}

/// Zero-copy dynamically-encoded DType backed by a borrowed byte slice.
pub struct DynBorrowedDType<'a> {
    pub(crate) bytes: &'a [u8],
}

impl<'a> dtype_sealed::Sealed for DynBorrowedDType<'a> {}

impl<'a> DynBorrowedDType<'a> {
    fn raw_decode_dtype(bytes: &'a [u8]) -> DTypeView<DynBorrowedDType<'a>, DynBorrowedDType<'a>> {
        assert!(!bytes.is_empty(), "Attempted to decode empty buffer");

        // Strip trailing NOPs, find opcode
        let (mut op, mut rest) = bytes.split_last().unwrap();
        while *op == MISC_NOP {
            (op, rest) = rest.split_last().unwrap();
        }
        let mut s: &[u8] = rest;

        use encoding::magic::*;
        match *op {
            T_BOOL => DTypeView::Bool,
            T_OMEGA => DTypeView::Omega,
            T_NEVER => DTypeView::Never,
            T_POWER => {
                // child: everything before opcode
                DTypeView::Power(DynBorrowedDType { bytes: s })
            }
            T_ARROW | T_TUPLE => {
                // Binary: A B len(B) OP
                let right_len = encoding::integer::decode_u64(&mut s).expect(
                    "Invalid encoding: expected length of right child before arrow/tuple opcode",
                );
                assert!(
                    right_len as usize <= s.len(),
                    "Invalid encoding: right length exceeds available bytes"
                );

                let split_at = s.len() - right_len as usize;
                let (left_bytes, right_bytes) = s.split_at(split_at);
                let l = DynBorrowedDType { bytes: left_bytes };
                let r = DynBorrowedDType { bytes: right_bytes };
                match *op {
                    T_ARROW => DTypeView::Arrow(l, r),
                    T_TUPLE => DTypeView::Tuple(l, r),
                    _ => unreachable!(),
                }
            }
            MISC_VAR => {
                let id = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected variable id after VAR_INLINE opcode");
                DTypeView::Var(InlineVariable::new_from_raw(id))
            }
            _ => panic!("Invalid encoding: unknown dtype opcode {}", *op),
        }
    }
}

impl<'a> DType for DynBorrowedDType<'a> {
    fn view_dtype(&self) -> DTypeView<impl DType, impl DType> {
        Self::raw_decode_dtype(self.bytes)
    }
}

impl<'a> RawEncodable for DynBorrowedDType<'a> {
    #[inline]
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&self.bytes);
        self.bytes.len() as u64
    }

    fn encoded_size(&self) -> u64 {
        self.bytes.len() as u64
    }
}

impl<'a> DynBorrowedDType<'a> {
    /// Decode with concrete borrowed types (no allocations).
    pub fn decode_dtype_concrete(&self) -> DTypeView<DynBorrowedDType<'a>, DynBorrowedDType<'a>> {
        Self::raw_decode_dtype(self.bytes)
    }
}
