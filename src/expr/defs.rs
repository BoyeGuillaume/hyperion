//! Concrete unified expression constructors: terms, logic, and types.
//!
//! All these types implement [`crate::expr::Expr`] and support encoding/decoding. They are
//! lightweight wrappers that provide structure; you normally compose them via the builder
//! methods on [`Expr`](crate::expr::Expr) or helpers in [`crate::expr::func`].
use crate::{
    arena::{ArenaAllocableExpr, ArenaAnyExpr, ExprArenaCtx},
    encoding::{
        EncodableExpr,
        tree::{TreeBuf, TreeBufNodeRef},
    },
    expr::{AnyExpr, Expr, variant::ExprType, view::ExprView},
    variable::InlineVariable,
};

// Lightweight operator sugar for logical combinations on expressions.
macro_rules! define_ops_expr {
    (
        $name:ident
        $( <
            $( $($lft:lifetime),+ $(,)? )?
            $( $($gen_name:ident: $gen:tt ),+ $(,)? )?
        > )?
    ) => {
        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
            _O1: Expr
        > std::ops::BitAnd<_O1> for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = And<Self, _O1>;

            fn bitand(self, rhs: _O1) -> Self::Output {
                And { lhs: self, rhs }
            }
        }

        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
            _O1: Expr
        > std::ops::BitOr<_O1> for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = Or<Self, _O1>;

            fn bitor(self, rhs: _O1) -> Self::Output {
                Or { lhs: self, rhs }
            }
        }

        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
        > std::ops::Not for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = Not<Self>;

            fn not(self) -> Self::Output {
                Not { inner: self }
            }
        }
    };
}

// ========================= Propositional variables =========================
/// Logical constant `true`.
///
/// Role: unit proposition that is always valid. Zero children, encodes to a single node.
#[derive(Clone, Copy)]
pub struct True;

impl EncodableExpr for True {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::True as u8, None, &[])
    }
}

impl<'a> ArenaAllocableExpr<'a> for True {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::True))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::True))
    }
}

impl Expr for True {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::True
    }
}

define_ops_expr! { True }

/// Logical constant `false`.
///
/// Role: contradiction proposition. Zero children, encodes to a single node.
#[derive(Clone, Copy)]
pub struct False;

impl EncodableExpr for False {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::False as u8, None, &[])
    }
}

impl<'a> ArenaAllocableExpr<'a> for False {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::False))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::False))
    }
}

impl Expr for False {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::False
    }
}

define_ops_expr! { False }

/// Boolean negation `!P`.
///
/// Role: unary logical connective.
#[derive(Clone, Copy)]
pub struct Not<P: Expr> {
    pub inner: P,
}

impl<P: Expr> EncodableExpr for Not<P> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let inner_ref = self.inner.encode_tree_step(tree);
        tree.push_node(ExprType::Not as u8, None, &[inner_ref])
    }
}

impl<'a, P: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a> for Not<P> {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let inner_alloc = self.inner.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(inner_alloc)))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let inner_alloc = self.inner.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(inner_alloc)))
    }
}

impl<P: Expr> Expr for Not<P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, AnyExpr, AnyExpr>::Not(&self.inner)
    }
}

define_ops_expr! { Not<P: Expr> }

/// Boolean conjunction `P /\ Q`.
///
/// Role: binary logical connective. Left and right children are expressions.
#[derive(Clone, Copy)]
pub struct And<P: Expr, Q: Expr> {
    pub lhs: P,
    pub rhs: Q,
}

impl<P: Expr, Q: Expr> EncodableExpr for And<P, Q> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.lhs.encode_tree_step(tree);
        let right_ref = self.rhs.encode_tree_step(tree);
        tree.push_node(ExprType::And as u8, None, &[left_ref, right_ref])
    }
}

impl<'a, P: Expr + ArenaAllocableExpr<'a>, Q: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for And<P, Q>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let lhs_alloc = self.lhs.alloc_in(ctx);
        let rhs_alloc = self.rhs.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::And(lhs_alloc, rhs_alloc)))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let lhs_alloc = self.lhs.alloc_in(ctx);
        let rhs_alloc = self.rhs.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::And(lhs_alloc, rhs_alloc)))
    }
}

impl<P: Expr, Q: Expr> Expr for And<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::And(&self.lhs, &self.rhs)
    }
}

define_ops_expr! { And<P: Expr, Q: Expr> }

/// Boolean disjunction `P \/ Q`.
///
/// Role: binary logical connective.
#[derive(Clone, Copy)]
pub struct Or<P: Expr, Q: Expr> {
    pub lhs: P,
    pub rhs: Q,
}

impl<P: Expr, Q: Expr> EncodableExpr for Or<P, Q> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.lhs.encode_tree_step(tree);
        let right_ref = self.rhs.encode_tree_step(tree);
        tree.push_node(ExprType::Or as u8, None, &[left_ref, right_ref])
    }
}

impl<'a, P: Expr + ArenaAllocableExpr<'a>, Q: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for Or<P, Q>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let lhs_alloc = self.lhs.alloc_in(ctx);
        let rhs_alloc = self.rhs.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Or(lhs_alloc, rhs_alloc)))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let lhs_alloc = self.lhs.alloc_in(ctx);
        let rhs_alloc = self.rhs.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Or(lhs_alloc, rhs_alloc)))
    }
}

impl<P: Expr, Q: Expr> Expr for Or<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Or(&self.lhs, &self.rhs)
    }
}

define_ops_expr! { Or<P: Expr, Q: Expr> }

/// Logical implication `P => Q` (right-associative at the parser level).
///
/// Role: binary logical connective.
#[derive(Clone, Copy)]
pub struct Implies<P: Expr, Q: Expr> {
    pub antecedent: P,
    pub consequent: Q,
}

impl<P: Expr, Q: Expr> EncodableExpr for Implies<P, Q> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.antecedent.encode_tree_step(tree);
        let right_ref = self.consequent.encode_tree_step(tree);
        tree.push_node(ExprType::Implies as u8, None, &[left_ref, right_ref])
    }
}

impl<'a, P: Expr + ArenaAllocableExpr<'a>, Q: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for Implies<P, Q>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let lhs_alloc = self.antecedent.alloc_in(ctx);
        let rhs_alloc = self.consequent.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Implies(
            lhs_alloc, rhs_alloc,
        )))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let lhs_alloc = self.antecedent.alloc_in(ctx);
        let rhs_alloc = self.consequent.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Implies(
            lhs_alloc, rhs_alloc,
        )))
    }
}

impl<P: Expr, Q: Expr> Expr for Implies<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Implies(&self.antecedent, &self.consequent)
    }
}

define_ops_expr! { Implies<P: Expr, Q: Expr> }

/// Logical equivalence `P <=> Q`.
///
/// Role: binary logical connective.
#[derive(Clone, Copy)]
pub struct Iff<P: Expr, Q: Expr> {
    pub lhs: P,
    pub rhs: Q,
}

impl<P: Expr, Q: Expr> EncodableExpr for Iff<P, Q> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.lhs.encode_tree_step(tree);
        let right_ref = self.rhs.encode_tree_step(tree);
        tree.push_node(ExprType::Iff as u8, None, &[left_ref, right_ref])
    }
}

impl<'a, P: Expr + ArenaAllocableExpr<'a>, Q: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for Iff<P, Q>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let lhs_alloc = self.lhs.alloc_in(ctx);
        let rhs_alloc = self.rhs.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Iff(lhs_alloc, rhs_alloc)))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let lhs_alloc = self.lhs.alloc_in(ctx);
        let rhs_alloc = self.rhs.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Iff(lhs_alloc, rhs_alloc)))
    }
}

impl<P: Expr, Q: Expr> Expr for Iff<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Iff(&self.lhs, &self.rhs)
    }
}

define_ops_expr! { Iff<P: Expr, Q: Expr> }

#[derive(Clone, Copy)]
/// Equality `A = B` between two expressions.
///
/// Role: binary relation across the unified language.
pub struct Equal<P: Expr, Q: Expr> {
    pub lhs: P,
    pub rhs: Q,
}

impl<P: Expr, Q: Expr> EncodableExpr for Equal<P, Q> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.lhs.encode_tree_step(tree);
        let right_ref = self.rhs.encode_tree_step(tree);
        tree.push_node(ExprType::Equal as u8, None, &[left_ref, right_ref])
    }
}

impl<'a, P: Expr + ArenaAllocableExpr<'a>, Q: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for Equal<P, Q>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let lhs_alloc = self.lhs.alloc_in(ctx);
        let rhs_alloc = self.rhs.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Equal(
            lhs_alloc, rhs_alloc,
        )))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let lhs_alloc = self.lhs.alloc_in(ctx);
        let rhs_alloc = self.rhs.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Equal(
            lhs_alloc, rhs_alloc,
        )))
    }
}

impl<P: Expr, Q: Expr> Expr for Equal<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Equal(&self.lhs, &self.rhs)
    }
}

define_ops_expr! { Equal<P: Expr, Q: Expr> }

// ======================== Quantified variables =========================
/// Universal quantification: `forall x : T . P`.
///
/// Role: binds a variable over a domain `T` and asserts `P` holds for all values.
#[derive(Clone, Copy)]
pub struct ForAll<DT: Expr, P: Expr> {
    pub variable: InlineVariable,
    pub dtype: DT,
    pub inner: P,
}

impl<DT: Expr, P: Expr> EncodableExpr for ForAll<DT, P> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let dtype_ref = self.dtype.encode_tree_step(tree);
        let inner_ref = self.inner.encode_tree_step(tree);
        tree.push_node(
            ExprType::Forall as u8,
            Some(self.variable.raw()),
            &[dtype_ref, inner_ref],
        )
    }
}

impl<'a, DT: Expr + ArenaAllocableExpr<'a>, P: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for ForAll<DT, P>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let dtype_alloc = self.dtype.alloc_in(ctx);
        let inner_alloc = self.inner.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Forall {
            variable: self.variable,
            dtype: dtype_alloc,
            inner: inner_alloc,
        }))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let dtype_alloc = self.dtype.alloc_in(ctx);
        let inner_alloc = self.inner.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Forall {
            variable: self.variable,
            dtype: dtype_alloc,
            inner: inner_alloc,
        }))
    }
}

impl<DT: Expr, P: Expr> Expr for ForAll<DT, P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&DT, &P, AnyExpr>::Forall {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
}

define_ops_expr! { ForAll<DT: Expr, P: Expr> }

/// Existential quantification: `exists x : T . P`.
///
/// Role: binds a variable and asserts existence of a witness.
#[derive(Clone, Copy)]
pub struct Exists<DT: Expr, P: Expr> {
    pub variable: InlineVariable,
    pub dtype: DT,
    pub inner: P,
}

impl<DT: Expr, P: Expr> EncodableExpr for Exists<DT, P> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let dtype_ref = self.dtype.encode_tree_step(tree);
        let inner_ref = self.inner.encode_tree_step(tree);
        tree.push_node(
            ExprType::Exists as u8,
            Some(self.variable.raw()),
            &[dtype_ref, inner_ref],
        )
    }
}

impl<'a, DT: Expr + ArenaAllocableExpr<'a>, P: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for Exists<DT, P>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let dtype_alloc = self.dtype.alloc_in(ctx);
        let inner_alloc = self.inner.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Exists {
            variable: self.variable,
            dtype: dtype_alloc,
            inner: inner_alloc,
        }))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let dtype_alloc = self.dtype.alloc_in(ctx);
        let inner_alloc = self.inner.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Exists {
            variable: self.variable,
            dtype: dtype_alloc,
            inner: inner_alloc,
        }))
    }
}

impl<DT: Expr, P: Expr> Expr for Exists<DT, P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&DT, &P, AnyExpr>::Exists {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
}

define_ops_expr! { Exists<DT: Expr, P: Expr> }

// ========================= Other expressions (not logic) =========================
// ========================= Constants =========================

/// Type constructor `Bool`.
///
/// Role: simple type for boolean values.
#[derive(Clone, Copy)]
pub struct Bool;

impl EncodableExpr for Bool {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Bool as u8, None, &[])
    }
}

impl<'a> ArenaAllocableExpr<'a> for Bool {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Bool))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Bool))
    }
}

impl Expr for Bool {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Bool
    }
}

/// Top type `Omega`.
///
/// Role: supertype of all terms in some encodings.
#[derive(Clone, Copy)]
pub struct Omega;

impl EncodableExpr for Omega {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Omega as u8, None, &[])
    }
}

impl<'a> ArenaAllocableExpr<'a> for Omega {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Omega))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Omega))
    }
}

impl Expr for Omega {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Omega
    }
}

/// Bottom term `<>` (never / unreachable).
///
/// Role: uninhabited term-level constant used in conditionals or as a sentinel.
#[derive(Clone, Copy)]
pub struct Never;

impl EncodableExpr for Never {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Never as u8, None, &[])
    }
}

impl<'a> ArenaAllocableExpr<'a> for Never {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Never))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Never))
    }
}

impl Expr for Never {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Never
    }
}

// ========================= Power set =========================
/// Powerset type `P(A)`.
///
/// Role: type-level unary constructor.
#[derive(Clone, Copy)]
pub struct Powerset<P: Expr> {
    pub inner: P,
}

impl<P: Expr> EncodableExpr for Powerset<P> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let inner_ref = self.inner.encode_tree_step(tree);
        tree.push_node(ExprType::Powerset as u8, None, &[inner_ref])
    }
}

impl<'a, P: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a> for Powerset<P> {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let inner_alloc = self.inner.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Powerset(inner_alloc)))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let inner_alloc = self.inner.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Powerset(inner_alloc)))
    }
}

impl<P: Expr> Expr for Powerset<P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, AnyExpr, AnyExpr>::Powerset(&self.inner)
    }
}

// ======================== Other binary expressions =========================
/// Lambda abstraction `arg -> body`.
///
/// Role: function-like constructor. At the term level it represents λ-calculus abstraction; at
/// the type level it can model function types when combined in conventions.
#[derive(Clone, Copy)]
pub struct Lambda<A: Expr, B: Expr> {
    pub arg: A,
    pub body: B,
}

impl<A: Expr, B: Expr> EncodableExpr for Lambda<A, B> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let arg_ref = self.arg.encode_tree_step(tree);
        let body_ref = self.body.encode_tree_step(tree);
        tree.push_node(ExprType::Lambda as u8, None, &[arg_ref, body_ref])
    }
}

impl<'a, A: Expr + ArenaAllocableExpr<'a>, B: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for Lambda<A, B>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let arg_alloc = self.arg.alloc_in(ctx);
        let body_alloc = self.body.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Lambda {
            arg: arg_alloc,
            body: body_alloc,
        }))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let arg_alloc = self.arg.alloc_in(ctx);
        let body_alloc = self.body.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Lambda {
            arg: arg_alloc,
            body: body_alloc,
        }))
    }
}

impl<A: Expr, B: Expr> Expr for Lambda<A, B> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, &B, AnyExpr>::Lambda {
            arg: &self.arg,
            body: &self.body,
        }
    }
}

/// Function application `func(arg)`.
///
/// Role: term-level application. Also used to apply type-level constructors when meaningful.
#[derive(Clone, Copy)]
pub struct Call<A: Expr, B: Expr> {
    pub func: A,
    pub arg: B,
}

impl<A: Expr, B: Expr> EncodableExpr for Call<A, B> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let func_ref = self.func.encode_tree_step(tree);
        let arg_ref = self.arg.encode_tree_step(tree);
        tree.push_node(ExprType::Call as u8, None, &[func_ref, arg_ref])
    }
}

impl<'a, A: Expr + ArenaAllocableExpr<'a>, B: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for Call<A, B>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let func_alloc = self.func.alloc_in(ctx);
        let arg_alloc = self.arg.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Call {
            func: func_alloc,
            arg: arg_alloc,
        }))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let func_alloc = self.func.alloc_in(ctx);
        let arg_alloc = self.arg.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Call {
            func: func_alloc,
            arg: arg_alloc,
        }))
    }
}

impl<A: Expr, B: Expr> Expr for Call<A, B> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, &B, AnyExpr>::Call {
            func: &self.func,
            arg: &self.arg,
        }
    }
}

/// Tuple `(A, B)` (also used as a type constructor).
///
/// Role: binary product type/term.
#[derive(Clone, Copy)]
pub struct Tuple<A: Expr, B: Expr> {
    pub first: A,
    pub second: B,
}

impl<A: Expr, B: Expr> EncodableExpr for Tuple<A, B> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let first_ref = self.first.encode_tree_step(tree);
        let second_ref = self.second.encode_tree_step(tree);
        tree.push_node(ExprType::Tuple as u8, None, &[first_ref, second_ref])
    }
}

impl<'a, A: Expr + ArenaAllocableExpr<'a>, B: Expr + ArenaAllocableExpr<'a>> ArenaAllocableExpr<'a>
    for Tuple<A, B>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let first_alloc = self.first.alloc_in(ctx);
        let second_alloc = self.second.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Tuple(
            first_alloc,
            second_alloc,
        )))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let first_alloc = self.first.alloc_in(ctx);
        let second_alloc = self.second.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Tuple(
            first_alloc,
            second_alloc,
        )))
    }
}

impl<A: Expr, B: Expr> Expr for Tuple<A, B> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, &B, AnyExpr>::Tuple(&self.first, &self.second)
    }
}

// ======================== If-then-else =========================
/// If-then-else conditional.
///
/// Role: ternary term-level construct: `if condition then then_branch else else_branch`.
#[derive(Clone, Copy)]
pub struct If<P: Expr, T: Expr, E: Expr> {
    pub condition: P,
    pub then_branch: T,
    pub else_branch: E,
}

impl<P: Expr, T: Expr, E: Expr> EncodableExpr for If<P, T, E> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let cond_ref = self.condition.encode_tree_step(tree);
        let then_ref = self.then_branch.encode_tree_step(tree);
        let else_ref = self.else_branch.encode_tree_step(tree);
        tree.push_node(ExprType::If as u8, None, &[cond_ref, then_ref, else_ref])
    }
}

impl<
    'a,
    P: Expr + ArenaAllocableExpr<'a>,
    T: Expr + ArenaAllocableExpr<'a>,
    E: Expr + ArenaAllocableExpr<'a>,
> ArenaAllocableExpr<'a> for If<P, T, E>
{
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        let cond_alloc = self.condition.alloc_in(ctx);
        let then_alloc = self.then_branch.alloc_in(ctx);
        let else_alloc = self.else_branch.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::If {
            condition: cond_alloc,
            then_branch: then_alloc,
            else_branch: else_alloc,
        }))
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        let cond_alloc = self.condition.alloc_in(ctx);
        let then_alloc = self.then_branch.alloc_in(ctx);
        let else_alloc = self.else_branch.alloc_in(ctx);
        ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::If {
            condition: cond_alloc,
            then_branch: then_alloc,
            else_branch: else_alloc,
        }))
    }
}

impl<P: Expr, T: Expr, E: Expr> Expr for If<P, T, E> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &T, &E>::If {
            condition: &self.condition,
            then_branch: &self.then_branch,
            else_branch: &self.else_branch,
        }
    }
}
