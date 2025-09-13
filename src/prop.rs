use crate::{
    dtype::DynBorrowedDType,
    encoding::{DynBuf, RawEncodable},
    expr::{DynBorrowedExpr, DynExpr, Expr, dispatch::ExprDispatch, expr_sealed},
    prop::dispatch::PropDispatch,
};

mod defs;
pub mod dispatch;
pub use defs::*;

pub(crate) mod prop_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait Prop: Expr + prop_sealed::Sealed + Sized + RawEncodable {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType>;

    /// Encode this proposition into a dynamic, byte-backed representation.
    #[inline]
    fn encode(&self) -> DynProp {
        let mut buf = DynBuf::new();
        self.encode_raw(&mut buf);
        DynProp { bytes: buf }
    }

    fn and<Q: Prop>(self, other: Q) -> And<Self, Q>
    where
        Self: Sized,
    {
        And {
            left: self,
            right: other,
        }
    }

    fn or<Q: Prop>(self, other: Q) -> Or<Self, Q>
    where
        Self: Sized,
    {
        Or {
            left: self,
            right: other,
        }
    }

    fn implies<Q: Prop>(self, other: Q) -> Imp<Self, Q>
    where
        Self: Sized,
    {
        Imp {
            antecedent: self,
            consequent: other,
        }
    }

    fn iff<Q: Prop>(self, other: Q) -> Iff<Self, Q>
    where
        Self: Sized,
    {
        Iff {
            left: self,
            right: other,
        }
    }

    fn not(self) -> Not<Self>
    where
        Self: Sized,
    {
        Not { inner: self }
    }
}

impl<'a, T: Prop> Prop for &'a T {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        (*self).decode_prop()
    }
}

/// Dynamically-encoded Prop backed by a compact byte buffer.
pub struct DynProp {
    pub(crate) bytes: DynBuf,
}

impl prop_sealed::Sealed for DynProp {}
impl expr_sealed::Sealed for DynProp {}

impl Expr for DynProp {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        ExprDispatch::<&Self, DynExpr, DynExpr>::Prop(self)
    }
}

impl Prop for DynProp {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        self.as_borrowed().decode_prop_concrete()
    }
}

impl RawEncodable for DynProp {
    #[inline]
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.extend_from_slice(&self.bytes);
    }
}

impl DynProp {
    /// Construct a dynamic proposition from raw encoded bytes.
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Self {
        let mut buf = DynBuf::new();
        buf.extend_from_slice(bytes.as_ref());
        Self { bytes: buf }
    }

    /// Borrow these bytes as a zero-copy dynamic proposition.
    #[inline]
    pub fn as_borrowed(&self) -> DynBorrowedProp<'_> {
        DynBorrowedProp {
            bytes: self.bytes.as_slice(),
        }
    }
}

/// Zero-copy dynamically-encoded Prop backed by a borrowed byte slice.
pub struct DynBorrowedProp<'a> {
    pub(crate) bytes: &'a [u8],
}

impl<'a> prop_sealed::Sealed for DynBorrowedProp<'a> {}
impl<'a> expr_sealed::Sealed for DynBorrowedProp<'a> {}

impl<'a> Expr for DynBorrowedProp<'a> {
    fn decode_expr(&self) -> ExprDispatch<impl Prop, impl Expr, impl Expr> {
        // Represent a Prop as an Expr without decoding; same as DynProp does.
        ExprDispatch::<&Self, DynBorrowedExpr<'a>, DynBorrowedExpr<'a>>::Prop(self)
    }
}

impl<'a> Prop for DynBorrowedProp<'a> {
    fn decode_prop(
        &self,
    ) -> PropDispatch<impl Prop, impl Prop, impl Expr, impl Expr, impl crate::dtype::DType> {
        self.decode_prop_concrete()
    }
}

impl<'a> RawEncodable for DynBorrowedProp<'a> {
    #[inline]
    fn encode_raw(&self, buf: &mut DynBuf) {
        buf.extend_from_slice(self.bytes);
    }
}

impl DynProp {
    /// Zero-copy decode returning borrowed children.
    #[inline]
    pub fn decode_prop_borrowed(
        &self,
    ) -> PropDispatch<
        DynBorrowedProp<'_>,
        DynBorrowedProp<'_>,
        DynBorrowedExpr<'_>,
        DynBorrowedExpr<'_>,
        DynBorrowedDType<'_>,
    > {
        self.as_borrowed().decode_prop_concrete()
    }
}

impl<'a> DynBorrowedProp<'a> {
    fn raw_decode_prop(
        bytes: &'a [u8],
    ) -> PropDispatch<
        DynBorrowedProp<'a>,
        DynBorrowedProp<'a>,
        DynBorrowedExpr<'a>,
        DynBorrowedExpr<'a>,
        DynBorrowedDType<'a>,
    > {
        if bytes.is_empty() {
            return PropDispatch::<
                DynBorrowedProp<'a>,
                DynBorrowedProp<'a>,
                DynBorrowedExpr<'a>,
                DynBorrowedExpr<'a>,
                DynBorrowedDType<'a>,
            >::True;
        }

        use crate::{encoding, encoding::magic::*, variable::InlineVariable};

        let (rest, op) = bytes.split_at(bytes.len() - 1);
        let mut s: &[u8] = rest;

        match op[0] {
            P_TRUE => PropDispatch::True,
            P_FALSE => PropDispatch::False,
            P_NOT => PropDispatch::Not(DynBorrowedProp { bytes: s }),
            P_AND | P_OR | P_IMPLIES | P_IFF => {
                // left right len(right) OP
                let right_len = encoding::integer::decode_u64(&mut s).expect(
                    "Invalid encoding: expected right-hand side length after binary operator",
                );
                assert!(
                    right_len as usize <= s.len(),
                    "Invalid encoding: right-hand side length exceeds available bytes"
                );

                let split_at = s.len() - right_len as usize;
                let (left_bytes, right_bytes) = s.split_at(split_at);
                let l = DynBorrowedProp { bytes: left_bytes };
                let r = DynBorrowedProp { bytes: right_bytes };
                match op[0] {
                    P_AND => PropDispatch::And(l, r),
                    P_OR => PropDispatch::Or(l, r),
                    P_IMPLIES => PropDispatch::Implies(l, r),
                    _ /* P_IFF */ => PropDispatch::Iff(l, r),
                }
            }
            P_FORALL | P_EXISTS => {
                // dtype inner len(inner) payload(var_id) OP
                let var_id = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected variable id after forall/exists opcode");
                let inner_len = encoding::integer::decode_u64(&mut s).expect(
                    "Invalid encoding: expected inner proposition length after variable id",
                );
                assert!(
                    (inner_len as usize) <= s.len(),
                    "Invalid encoding: inner proposition length exceeds available bytes"
                );

                let split_at = s.len() - inner_len as usize;
                let (dtype_bytes, inner_bytes) = s.split_at(split_at);
                let dt = DynBorrowedDType { bytes: dtype_bytes };
                let inner = DynBorrowedProp { bytes: inner_bytes };
                let var = InlineVariable::new(var_id);
                match op[0] {
                    P_FORALL => PropDispatch::ForAll { variable: var, dtype: dt, inner },
                    _ /* P_EXISTS */ => PropDispatch::Exists { variable: var, dtype: dt, inner },
                }
            }
            P_EQUAL => {
                let right_len = encoding::integer::decode_u64(&mut s).expect(
                    "Invalid encoding: expected right-hand side length after equality operator",
                );
                assert!(
                    right_len as usize <= s.len(),
                    "Invalid encoding: right-hand side length exceeds available bytes"
                );

                let split_at = s.len() - right_len as usize;
                let (left_bytes, right_bytes) = s.split_at(split_at);
                let l = DynBorrowedExpr { bytes: left_bytes };
                let r = DynBorrowedExpr { bytes: right_bytes };
                PropDispatch::Equal(l, r)
            }
            _ => panic!("Invalid encoding: unknown prop opcode {}", op[0]),
        }
    }

    /// Decode with concrete borrowed types (no allocations).
    #[inline]
    pub fn decode_prop_concrete(
        &self,
    ) -> PropDispatch<
        DynBorrowedProp<'a>,
        DynBorrowedProp<'a>,
        DynBorrowedExpr<'a>,
        DynBorrowedExpr<'a>,
        DynBorrowedDType<'a>,
    > {
        Self::raw_decode_prop(self.bytes)
    }
}
