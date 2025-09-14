pub mod defs;
pub mod dispatch;
pub use defs::*;

use crate::dtype::dispatch::DTypeDispatch;
use crate::encoding::{DynBuf, RawEncodable};
use crate::{encoding, variable::InlineVariable};

pub(crate) mod dtype_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait DType: dtype_sealed::Sealed + Sized + RawEncodable {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType>;

    /// Encode this type into a dynamic, byte-backed representation.
    #[inline]
    fn encode(&self) -> DynDType {
        let mut buf = DynBuf::new();
        self.encode_raw(&mut buf);
        DynDType { bytes: buf }
    }

    #[inline]
    fn application<Q: DType>(self, arg: Q) -> defs::TApp<Self, Q> {
        defs::TApp {
            from: self,
            to: arg,
        }
    }

    #[inline]
    fn app<Q: DType>(self, arg: Q) -> defs::TApp<Self, Q> {
        self.application(arg)
    }

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

    #[inline]
    fn powerset(self) -> defs::TPowerSet<Self>
    where
        Self: Sized,
    {
        defs::TPowerSet { inner: self }
    }
}

impl<'a, T: DType> DType for &'a T {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        (*self).decode_dtype()
    }
}

/// Dynamically-encoded DType backed by a compact byte buffer.
pub struct DynDType {
    pub(crate) bytes: DynBuf,
}
impl dtype_sealed::Sealed for DynDType {}

impl DType for DynDType {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        DynBorrowedDType::raw_decode_dtype(&self.bytes)
    }
}

impl RawEncodable for DynDType {
    #[inline]
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.extend_from_slice(&self.bytes);
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
    pub fn decode_dtype_concrete(
        &self,
    ) -> DTypeDispatch<DynBorrowedDType<'_>, DynBorrowedDType<'_>> {
        self.as_borrowed().decode_dtype_concrete()
    }
}

/// Zero-copy dynamically-encoded DType backed by a borrowed byte slice.
pub struct DynBorrowedDType<'a> {
    pub(crate) bytes: &'a [u8],
}

impl<'a> dtype_sealed::Sealed for DynBorrowedDType<'a> {}

impl<'a> DynBorrowedDType<'a> {
    fn raw_decode_dtype(
        bytes: &'a [u8],
    ) -> DTypeDispatch<DynBorrowedDType<'a>, DynBorrowedDType<'a>> {
        assert!(!bytes.is_empty(), "Attempted to decode empty buffer");

        let (rest, op) = bytes.split_at(bytes.len() - 1);
        let mut s: &[u8] = rest;

        use encoding::magic::*;
        match op[0] {
            T_BOOL => DTypeDispatch::Bool,
            T_OMEGA => DTypeDispatch::Omega,
            T_NEVER => DTypeDispatch::Never,
            T_POWER => {
                // child: everything before opcode
                DTypeDispatch::Power(DynBorrowedDType { bytes: s })
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
                match op[0] {
                    T_ARROW => DTypeDispatch::Arrow(l, r),
                    _ /* T_TUPLE */ => DTypeDispatch::Tuple(l, r),
                }
            }
            VAR_INLINE => {
                let id = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected variable id after VAR_INLINE opcode");
                DTypeDispatch::Var(InlineVariable::new(id))
            }
            _ => panic!("Invalid encoding: unknown dtype opcode {}", op[0]),
        }
    }
}

impl<'a> DType for DynBorrowedDType<'a> {
    fn decode_dtype(&self) -> DTypeDispatch<impl DType, impl DType> {
        Self::raw_decode_dtype(self.bytes)
    }
}

impl<'a> RawEncodable for DynBorrowedDType<'a> {
    #[inline]
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.extend_from_slice(self.bytes);
    }
}

impl<'a> DynBorrowedDType<'a> {
    /// Decode with concrete borrowed types (no allocations).
    pub fn decode_dtype_concrete(
        &self,
    ) -> DTypeDispatch<DynBorrowedDType<'a>, DynBorrowedDType<'a>> {
        Self::raw_decode_dtype(self.bytes)
    }
}
