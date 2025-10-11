//! Unified expressions: constructors and zero-copy dynamic decoding.
//!
//! Role
//! - Provide a single expression language covering types, terms, and logic.
//! - Builders in [`defs`] and helpers in [`func`] let you create expressions ergonomically.
//! - [`Expr::encode`] produces a compact owned buffer, and [`AnyExprRef`] lets you borrow
//!   and traverse without allocating.
//!
//! Performance
//! - Building is O(n) across nodes, with small buffers inlined before spilling to the heap.
//! - `view()` decodes the outer constructor in O(1); traversal is allocation-free.
//!
//! Example
//! ```
//! use hyformal::expr::*;
//! use hyformal::expr::defs::{Bool, True};
//! use hyformal::variable::InlineVariable;
//!
//! let x = InlineVariable::new_from_raw(0);
//! let lam = x.lambda(True);
//! let encoded = lam.encode();
//! assert!(matches!(encoded.as_ref().view().type_(), variant::ExprType::Lambda));
//! ```
pub mod defs;
pub mod func;
pub mod pretty;
pub mod variant;
pub mod view;

use crate::encoding::EncodableExpr;
use crate::encoding::tree::{TreeBuf, TreeBufNodeRef};
use crate::expr::variant::ExprType;
use crate::expr::view::ExprView;
use crate::prelude::{Call, Equal, Lambda};
use crate::utils::staticvec::StaticVec;
use crate::variable::InlineVariable;

pub(crate) mod expr_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

/// Trait implemented by all unified expression nodes provided by this crate.
///
/// Role
/// - Unifies building and structural inspection across concrete node types (see [`defs`]).
/// - Provides sugar helpers like [`tuple`], [`powerset`], [`equals`], [`apply`], [`lambda`]
///   available on any `Expr`.
///
/// Performance
/// - [`view`] returns a cheap, decoded wrapper exposing children; no allocations.
/// - [`encode`] is linear in the size of the expression and may trigger consolidation.
pub trait Expr: expr_sealed::Sealed + Sized + EncodableExpr {
    /// Describe the expression's outer constructor and expose children.
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr>;

    /// Encode this expr into an owned, compact [`AnyExpr`].
    ///
    /// Complexity: O(n) in the number of nodes; may consolidate the buffer.
    #[inline]
    fn encode(&self) -> AnyExpr {
        let mut tree = TreeBuf::new();
        let root = self.encode_tree_step(&mut tree);
        tree.set_root(root);
        tree.consolite_if_needed();
        AnyExpr { tree }
    }

    /// Construct a tuple, can be either a type or a term based on context.
    #[inline]
    fn tuple<Q: Expr>(self, other: Q) -> defs::Tuple<Self, Q> {
        defs::Tuple {
            first: self,
            second: other,
        }
    }

    /// Construct the powerset type `P(self)` as a type-level expression.
    #[inline]
    fn powerset(self) -> defs::Powerset<Self> {
        defs::Powerset { inner: self }
    }

    /// Build an equality expression `self == other`.
    #[inline]
    fn equals<Q: Expr>(self, other: Q) -> Equal<Self, Q> {
        Equal {
            lhs: self,
            rhs: other,
        }
    }

    /// Call this body with the given argument: `self(other)`.
    #[inline]
    fn apply<Q: Expr>(self, arg: Q) -> Call<Self, Q> {
        Call { func: self, arg }
    }

    /// Lambda abstraction: `λ self. body`.
    #[inline]
    fn lambda<Q: Expr>(self, body: Q) -> Lambda<Self, Q> {
        Lambda { arg: self, body }
    }
}

impl<'a, T: Expr> Expr for &'a T {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        (*self).view()
    }
}

/// Owned, compactly-encoded expression.
///
/// Role
/// - Stores the encoded tree in a [`TreeBuf`]. Use [`AnyExpr::as_ref`] to borrow a zero-copy view.
#[derive(Clone)]
pub struct AnyExpr {
    pub(crate) tree: TreeBuf,
}

impl AnyExpr {
    /// Borrow this expression as an [`AnyExprRef`].
    ///
    /// Useful for zero-copy traversal and pretty-printing without cloning the buffer.
    pub(crate) fn _view(
        tree: &TreeBuf,
        node: TreeBufNodeRef,
    ) -> ExprView<AnyExprRef<'_>, AnyExprRef<'_>, AnyExprRef<'_>> {
        let (opcode, data, children) = tree.get_node(node);
        let children: StaticVec<_, 3> = children
            .into_iter()
            .map(|c| AnyExprRef { tree, node: c })
            .collect();
        let expr_type = ExprType::from_repr(opcode).unwrap();
        use ExprView::*;
        debug_assert!(
            data.is_some()
                == matches!(
                    expr_type,
                    ExprType::Variable | ExprType::Forall | ExprType::Exists
                ),
            "Expected data for Variable, Forall, and Exists nodes only (got {:?} with data {:?})",
            expr_type,
            data
        );
        debug_assert!(
            children.len()
                == match expr_type {
                    ExprType::Bool
                    | ExprType::Omega
                    | ExprType::True
                    | ExprType::False
                    | ExprType::Never => 0,
                    ExprType::Not | ExprType::Powerset => 1,
                    ExprType::And
                    | ExprType::Or
                    | ExprType::Implies
                    | ExprType::Iff
                    | ExprType::Equal
                    | ExprType::Lambda
                    | ExprType::Call
                    | ExprType::Tuple => 2,
                    ExprType::Forall | ExprType::Exists => 2,
                    ExprType::If => 3,
                    ExprType::Variable => 0,
                },
            "Expected correct number of children for {:?} node, got {}",
            expr_type,
            children.len()
        );

        match expr_type {
            ExprType::Bool => Bool,
            ExprType::Omega => Omega,
            ExprType::True => True,
            ExprType::False => False,
            ExprType::Never => Never,
            ExprType::Not => Not(children[0]),
            ExprType::Powerset => Powerset(children[0]),
            ExprType::And => And(children[0], children[1]),
            ExprType::Or => Or(children[0], children[1]),
            ExprType::Implies => Implies(children[0], children[1]),
            ExprType::Iff => Iff(children[0], children[1]),
            ExprType::Equal => Equal(children[0], children[1]),
            ExprType::Lambda => Lambda {
                arg: children[0],
                body: children[1],
            },
            ExprType::Call => Call {
                func: children[0],
                arg: children[1],
            },
            ExprType::Tuple => Tuple(children[0], children[1]),
            ExprType::Forall => Forall {
                variable: InlineVariable::new_from_raw(data.unwrap()),
                dtype: children[0],
                inner: children[1],
            },
            ExprType::Exists => Exists {
                variable: InlineVariable::new_from_raw(data.unwrap()),
                dtype: children[0],
                inner: children[1],
            },
            ExprType::If => If {
                condition: children[0],
                then_branch: children[1],
                else_branch: children[2],
            },
            ExprType::Variable => Variable(InlineVariable::new_from_raw(data.unwrap())),
        }
    }

    /// Create a borrowed reference to the root node of this encoded expression.
    pub fn as_ref(&self) -> AnyExprRef<'_> {
        AnyExprRef {
            tree: &self.tree,
            node: self.tree.root().unwrap(),
        }
    }

    /// Length of underlying storage
    pub fn storage_size(&self) -> usize {
        self.tree.total_bytes()
    }

    /// Estimated number of bytes that can be reclaimed by consolidation.
    pub fn estimated_wasted_bytes(&self) -> usize {
        self.tree.wasted_bytes()
    }

    /// Consolidate the internal buffer if it is fragmented. This invalidates any existing handles however this
    /// is already enforced by borrowing rules.
    pub fn consolidate(&mut self) {
        self.tree.consolite_if_needed();
    }
}

impl expr_sealed::Sealed for AnyExpr {}
impl EncodableExpr for AnyExpr {
    fn encode_tree_step(
        self,
        tree: &mut crate::encoding::tree::TreeBuf,
    ) -> crate::encoding::tree::TreeBufNodeRef {
        tree.push_tree(&self.tree, self.tree.root().unwrap())
    }
}

impl Expr for AnyExpr {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        Self::_view(&self.tree, self.tree.root().unwrap())
    }
}

/// Borrowed reference to an encoded expression node.
///
/// Role
/// - Small handle pointing into an [`AnyExpr`] buffer; cheap to copy and pass around.
#[derive(Clone, Copy)]
pub struct AnyExprRef<'a> {
    pub(crate) tree: &'a TreeBuf,
    pub(crate) node: TreeBufNodeRef,
}

impl<'a> expr_sealed::Sealed for AnyExprRef<'a> {}

impl<'a> EncodableExpr for AnyExprRef<'a> {
    fn encode_tree_step(
        self,
        tree: &mut crate::encoding::tree::TreeBuf,
    ) -> crate::encoding::tree::TreeBufNodeRef {
        tree.push_tree(&self.tree, self.tree.root().unwrap())
    }
}

impl<'a> Expr for AnyExprRef<'a> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        AnyExpr::_view(self.tree, self.node)
    }
}
