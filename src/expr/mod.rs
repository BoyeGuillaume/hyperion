//! Unified expressions: constructors and zero-copy dynamic decoding.
//!
//! Build unified expressions (types, terms, and logic), encode to a compact buffer,
//! and decode later as borrowed views without allocations.
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
pub trait Expr: expr_sealed::Sealed + Sized + EncodableExpr {
    /// Describe the expression's outer constructor and expose children.
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr>;

    /// Encode this expr into an AnyExpr.
    #[inline]
    fn encode(&self) -> AnyExpr {
        let mut tree = TreeBuf::new();
        let root = self.encode_tree_step(&mut tree);
        tree.set_root(root);
        tree.consolite_if_needed();
        AnyExpr { tree }
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

    /// Lambda abstraction: `λ self. body`
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

#[derive(Clone)]
pub struct AnyExpr {
    pub(crate) tree: TreeBuf,
}

impl AnyExpr {
    fn _view(
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

    pub fn as_ref(&self) -> AnyExprRef<'_> {
        AnyExprRef {
            tree: &self.tree,
            node: self.tree.root().unwrap(),
        }
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
