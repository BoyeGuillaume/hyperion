//! Unified expressions: constructors and zero-copy dynamic decoding.
//!
//! Build unified expressions (types, terms, and logic), encode to a compact buffer,
//! and decode later as borrowed views without allocations.
pub mod defs;
pub mod func;
mod pretty;
pub mod view;
use crate::encoding::{DynBuf, RawEncodable};
use crate::expr::view::ExprView;
use crate::{encoding, variable::InlineVariable};

pub(crate) mod expr_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

/// Trait implemented by all unified expression nodes provided by this crate.
pub trait Expr: expr_sealed::Sealed + Sized + RawEncodable {
    /// Describe the expression's outer constructor and expose children.
    fn view_expr(&self) -> ExprView<impl Expr, impl Expr, impl Expr>;

    /// Encode this expr into a dynamic, byte-backed representation.
    #[inline]
    fn encode(&self) -> DynExpr {
        let mut buf = DynBuf::new();
        self.encode_dynbuf(&mut buf);
        DynExpr { bytes: buf }
    }

    /// Construct a function type from `self` to `arg` as a type-level expression.
    #[inline]
    fn func<Q: Expr>(self, codomain: Q) -> defs::Func<Self, Q> {
        defs::Func {
            domain: self,
            codomain,
        }
    }

    /// Construct a tuple, can be either a type or a term based on context
    #[inline]
    fn tuple<Q: Expr>(self, other: Q) -> defs::Tuple<Self, Q> {
        defs::Tuple {
            first: self,
            second: other,
        }
    }

    /// Construct the powerset type `P(self)` as a type-level expression.
    #[inline]
    fn powerset(self) -> defs::PowerSet<Self> {
        defs::PowerSet { inner: self }
    }

    #[inline]
    /// Build an equality expression `self == other`.
    fn equals<Q: Expr>(self, other: Q) -> defs::Eq<Self, Q>
    where
        Self: Sized,
    {
        defs::Eq {
            left: self,
            right: other,
        }
    }

    #[inline]
    /// Build a tuple expression `(self, other)`.
    fn make_tuple<Q: Expr>(self, other: Q) -> defs::Tuple<Self, Q>
    where
        Self: Sized,
    {
        defs::Tuple {
            first: self,
            second: other,
        }
    }

    // ===================== Pretty printing helpers =====================

    /// Build an RcDoc representation of this expression with style annotations.
    /// Useful for composing or rendering manually.
    #[inline]
    fn pretty_doc(&self) -> ::pretty::RcDoc<'static, crate::expr::pretty::Style> {
        crate::expr::pretty::to_doc_with_depth(self, 0)
    }

    /// Render this expression with colors to any termcolor writer at the given width.
    #[inline]
    fn pretty_render_to<W: ::termcolor::WriteColor + ::std::io::Write>(
        &self,
        width: usize,
        out: &mut W,
    ) -> ::std::io::Result<()> {
        let doc = self.pretty_doc();
        crate::expr::pretty::render_to(&doc, width, out)
    }

    /// Print this expression to stdout with colors (TTY-aware), at the given width.
    #[inline]
    fn pretty_print_with_width(&self, width: usize) -> ::std::io::Result<()> {
        crate::expr::pretty::print_colored(self, width)
    }

    /// Print this expression to stdout with colors (TTY-aware), at auto-detected width (or 80 if not a TTY).
    #[inline]
    fn pretty_print(&self) -> ::std::io::Result<()> {
        let width = crate::expr::pretty::terminal_width();
        crate::expr::pretty::print_colored(self, width)
    }

    /// Format this expression into a plain string (no colors), at the given width.
    #[inline]
    fn pretty_string_with_width(&self, width: usize) -> String {
        crate::expr::pretty::to_plain_string(self, width)
    }

    /// Format this expression into a plain string (no colors), at auto-detected width (or 80 if not a TTY).
    #[inline]
    fn pretty_string(&self) -> String {
        let width = crate::expr::pretty::terminal_width();
        crate::expr::pretty::to_plain_string(self, width)
    }
}

impl<'a, T: Expr> Expr for &'a T {
    fn view_expr(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        (*self).view_expr()
    }
}

/// Dynamically-encoded Expr backed by a compact byte buffer.
pub struct DynExpr {
    pub(crate) bytes: DynBuf,
}
impl expr_sealed::Sealed for DynExpr {}
impl Expr for DynExpr {
    fn view_expr(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
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
    ) -> ExprView<DynBorrowedExpr<'_>, DynBorrowedExpr<'_>, DynBorrowedExpr<'_>> {
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
    ) -> ExprView<DynBorrowedExpr<'a>, DynBorrowedExpr<'a>, DynBorrowedExpr<'a>> {
        assert!(!bytes.is_empty(), "Attempted to decode empty buffer");

        // Strip trailing NOPs, find opcode
        let (mut op, mut rest) = bytes.split_last().unwrap();
        while *op == MISC_NOP {
            (op, rest) = rest.split_last().unwrap();
        }
        let mut s: &[u8] = rest;

        use encoding::magic::*;
        match *op {
            // Term-level
            E_UNREACHABLE => ExprView::Never,
            E_APP => {
                let func_id = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected function id after E_APP opcode");
                ExprView::App {
                    func: InlineVariable::new_from_raw(func_id),
                    arg: DynBorrowedExpr { bytes: s },
                }
            }
            E_IF => {
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
                    condition: DynBorrowedExpr { bytes: cond_bytes },
                    then_branch: DynBorrowedExpr { bytes: then_bytes },
                    else_branch: DynBorrowedExpr { bytes: else_bytes },
                }
            }
            E_TUPLE => {
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

            // Shared var (context-free in unified language)
            MISC_VAR => {
                let id = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected variable id after VAR_INLINE opcode");
                ExprView::Var(InlineVariable::new_from_raw(id))
            }

            // Logic-level
            P_TRUE => ExprView::True,
            P_FALSE => ExprView::False,
            P_NOT => ExprView::Not(DynBorrowedExpr { bytes: s }),
            P_AND | P_OR | P_IMPLIES | P_IFF => {
                let right_len = encoding::integer::decode_u64(&mut s).expect(
                    "Invalid encoding: expected right-hand side length after binary operator",
                );
                assert!(
                    right_len as usize <= s.len(),
                    "Invalid encoding: right-hand side length exceeds available bytes"
                );
                let split_at = s.len() - right_len as usize;
                let (left_bytes, right_bytes) = s.split_at(split_at);
                let l = DynBorrowedExpr { bytes: left_bytes };
                let r = DynBorrowedExpr { bytes: right_bytes };
                match *op {
                    P_AND => ExprView::And(l, r),
                    P_OR => ExprView::Or(l, r),
                    P_IMPLIES => ExprView::Implies(l, r),
                    P_IFF => ExprView::Iff(l, r),
                    _ => unreachable!(),
                }
            }
            P_FORALL | P_EXISTS => {
                let var_id = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected variable id after forall/exists opcode");
                let inner_len = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected inner length after variable id");
                assert!(
                    (inner_len as usize) <= s.len(),
                    "Invalid encoding: inner length exceeds available bytes"
                );
                let split_at = s.len() - inner_len as usize;
                let (dtype_bytes, inner_bytes) = s.split_at(split_at);
                let dt = DynBorrowedExpr { bytes: dtype_bytes };
                let inner = DynBorrowedExpr { bytes: inner_bytes };
                let var = InlineVariable::new_from_raw(var_id);
                match *op {
                    P_FORALL => ExprView::ForAll {
                        variable: var,
                        dtype: dt,
                        inner,
                    },
                    P_EXISTS => ExprView::Exists {
                        variable: var,
                        dtype: dt,
                        inner,
                    },
                    _ => unreachable!(),
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
                ExprView::Equal(l, r)
            }

            // Type-level
            T_BOOL => ExprView::Bool,
            T_OMEGA => ExprView::Omega,
            T_NEVER => ExprView::Never,
            T_FUNC => {
                let right_len = encoding::integer::decode_u64(&mut s)
                    .expect("Invalid encoding: expected length of right child before func opcode");
                assert!(
                    right_len as usize <= s.len(),
                    "Invalid encoding: right length exceeds available bytes"
                );
                let split_at = s.len() - right_len as usize;
                let (left_bytes, right_bytes) = s.split_at(split_at);
                let l = DynBorrowedExpr { bytes: left_bytes };
                let r = DynBorrowedExpr { bytes: right_bytes };
                ExprView::Func(l, r)
            }
            // T_TUPLE => merged with E_TUPLE
            T_POWER => ExprView::Powerset(DynBorrowedExpr { bytes: s }),
            _ => panic!("Invalid encoding: unknown opcode {}", *op),
        }
    }

    /// Decode with concrete borrowed types (no allocations).
    #[inline]
    pub fn decode_expr_concrete(
        &self,
    ) -> ExprView<DynBorrowedExpr<'a>, DynBorrowedExpr<'a>, DynBorrowedExpr<'a>> {
        Self::raw_decode_expr(self.bytes)
    }
}

impl<'a> Expr for DynBorrowedExpr<'a> {
    fn view_expr(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
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
