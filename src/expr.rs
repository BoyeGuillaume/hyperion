//! Expression constructors and zero-copy dynamic expressions.
//!
//! Build expressions with typed helpers, encode them to a compact buffer, and
//! decode later as borrowed views without allocations.
pub mod defs;
pub mod view;
use crate::encoding::{DynBuf, RawEncodable};
use crate::expr::view::ExprView;
use crate::prop::{DynBorrowedProp, Eq, Prop};
use crate::{encoding, variable::InlineVariable};

pub(crate) mod expr_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

/// Trait implemented by all expression nodes provided by this crate.
pub trait Expr: expr_sealed::Sealed + Sized + RawEncodable {
    /// Describe the expression's outer constructor and expose children.
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr>;

    /// Encode this expr into a dynamic, byte-backed representation.
    #[inline]
    fn encode(&self) -> DynExpr {
        let mut buf = DynBuf::new();
        self.encode_dynbuf(&mut buf);
        DynExpr { bytes: buf }
    }

    #[inline]
    /// Build an equality proposition `self == other`.
    fn equals<Q: Expr>(self, other: Q) -> Eq<Self, Q>
    where
        Self: Sized,
    {
        Eq {
            left: self,
            right: other,
        }
    }

    #[inline]
    /// Build a tuple expression `(self, other)`.
    fn make_tuple<Q: Expr>(self, other: Q) -> defs::ETuple<Self, Q>
    where
        Self: Sized,
    {
        defs::ETuple {
            first: self,
            second: other,
        }
    }
}

impl<'a, T: Expr> Expr for &'a T {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        (*self).view_expr()
    }
}

/// Dynamically-encoded Expr backed by a compact byte buffer.
pub struct DynExpr {
    pub(crate) bytes: DynBuf,
}
impl expr_sealed::Sealed for DynExpr {}
impl Expr for DynExpr {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        self.as_borrowed().decode_expr_concrete()
    }
}

impl RawEncodable for DynExpr {
    #[inline]
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&self.bytes);
        self.bytes.len() as u64
    }

    fn encoded_size(&self) -> u64 {
        self.bytes.len() as u64
    }
}

impl DynExpr {
    /// Borrow these bytes as a zero-copy dynamic expr.
    #[inline]
    pub fn as_borrowed(&self) -> DynBorrowedExpr<'_> {
        DynBorrowedExpr {
            bytes: self.bytes.as_slice(),
        }
    }

    /// Zero-copy decode returning borrowed children.
    #[inline]
    pub fn decode_expr_borrowed(
        &self,
    ) -> ExprView<DynBorrowedProp<'_>, DynBorrowedExpr<'_>, DynBorrowedExpr<'_>> {
        self.as_borrowed().decode_expr_concrete()
    }
}

/// Zero-copy dynamically-encoded Expr backed by a borrowed byte slice.
pub struct DynBorrowedExpr<'a> {
    pub(crate) bytes: &'a [u8],
}

impl<'a> expr_sealed::Sealed for DynBorrowedExpr<'a> {}

impl<'a> DynBorrowedExpr<'a> {
    fn raw_decode_expr(
        bytes: &'a [u8],
    ) -> ExprView<DynBorrowedProp<'a>, DynBorrowedExpr<'a>, DynBorrowedExpr<'a>> {
        assert!(!bytes.is_empty(), "Attempted to decode empty buffer");

        // Strip trailing NOPs, find opcode
        let (mut op, mut rest) = bytes.split_last().unwrap();
        while *op == MISC_NOP {
            (op, rest) = rest.split_last().unwrap();
        }
        let mut s: &[u8] = rest;

        use encoding::magic::*;
        match *op {
            E_UNREACHABLE => ExprView::Unreachable,
            E_APP => {
                // arg payload(func_id) OP
                let func_id = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected function id after E_APP opcode");
                ExprView::App {
                    func: InlineVariable::new_from_raw(func_id),
                    arg: DynBorrowedExpr { bytes: s },
                }
            }
            E_IF => {
                // cond then else len(else) len(then) OP
                let then_len = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected then-branch length after E_IF opcode");
                let else_len = encoding::integer::decode_u64(&mut s).expect(
                    "Invalid encoding: expected else-branch length after then-branch length",
                );
                assert!(
                    (then_len as usize) + (else_len as usize) <= s.len(),
                    "Invalid encoding: then-branch or else-branch length exceeds available bytes"
                );

                let slen = s.len();
                let (prefix, else_bytes) = s.split_at(slen - else_len as usize);
                let (cond_bytes, then_bytes) = prefix.split_at(prefix.len() - then_len as usize);

                ExprView::If {
                    condition: DynBorrowedProp { bytes: cond_bytes },
                    then_branch: DynBorrowedExpr { bytes: then_bytes },
                    else_branch: DynBorrowedExpr { bytes: else_bytes },
                }
            }
            E_TUPLE => {
                // Binary: A B len(B) OP
                let right_len = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected length of right child before tuple opcode");
                assert!(
                    right_len as usize <= s.len(),
                    "Invalid encoding: right length exceeds available bytes"
                );

                let split_at = s.len() - right_len as usize;
                let (left_bytes, right_bytes) = s.split_at(split_at);
                ExprView::Tuple(
                    DynBorrowedExpr { bytes: left_bytes },
                    DynBorrowedExpr { bytes: right_bytes },
                )
            }
            MISC_VAR => {
                let id = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected variable id after VAR_INLINE opcode");

                ExprView::Var(InlineVariable::new_from_raw(id))
            }
            P_TRUE | P_FALSE | P_NOT | P_AND | P_OR | P_IMPLIES | P_IFF | P_FORALL | P_EXISTS
            | P_EQUAL => ExprView::Prop(DynBorrowedProp { bytes }),
            _ => panic!("Invalid encoding: unknown expr opcode {}", *op),
        }
    }

    /// Decode with concrete borrowed types (no allocations).
    #[inline]
    pub fn decode_expr_concrete(
        &self,
    ) -> ExprView<DynBorrowedProp<'a>, DynBorrowedExpr<'a>, DynBorrowedExpr<'a>> {
        Self::raw_decode_expr(self.bytes)
    }
}

impl<'a> Expr for DynBorrowedExpr<'a> {
    fn view_expr(&self) -> ExprView<impl Prop, impl Expr, impl Expr> {
        Self::raw_decode_expr(self.bytes)
    }
}

impl<'a> RawEncodable for DynBorrowedExpr<'a> {
    #[inline]
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        f(&self.bytes);
        self.bytes.len() as u64
    }

    fn encoded_size(&self) -> u64 {
        self.bytes.len() as u64
    }
}
