use either::Either;
use smallvec::SmallVec;
use typed_arena::Arena;

use crate::{
    encoding::{EncodableExpr, tree::TreeBufNodeRef},
    expr::{AnyExprRef, Expr, variant::ExprType, view::ExprView},
};

/// Arena-backed expressions for building and transforming trees without allocation churn.
///
/// Role
/// - Provide a temporary arena (`typed_arena`) to allocate lightweight expression nodes
///   while constructing or rewriting expressions.
/// - Mix and match fully-encoded borrowed subtrees ([`AnyExprRef`](crate::expr::AnyExprRef)) with
///   arena-local views ([`ExprView`](crate::expr::view::ExprView)) via [`ArenaAnyExpr`].
/// - Encode the final result into a compact owned buffer using the regular
///   [`EncodableExpr`](crate::encoding::EncodableExpr) API.
///
/// When to use
/// - You want to assemble or transform expressions from pieces without incurring many small
///   heap allocations or cloning intermediate owned buffers.
/// - You need a short-lived context to collect nodes and finally produce an owned
///   [`AnyExpr`](crate::expr::AnyExpr) at the end.
///
/// Example: building and encoding with an arena
/// ```
/// use hyformal::arena::{ExprArenaCtx, ArenaAnyExpr};
/// use hyformal::expr::{Expr, view::ExprView};
/// use hyformal::expr::defs::{True, False};
///
/// let ctx = ExprArenaCtx::new();
/// let t = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::True));
/// let f = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::False));
/// let and = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::And(t, f)));
///
/// let encoded = and.encode();
/// assert_eq!(encoded.as_ref().view().r#type(), hyformal::expr::variant::ExprType::And);
/// ```
///
/// Trait for values that can be allocated into an expression arena.
///
/// Contract
/// - Implementors produce arena-lifetime values (`&'a ArenaAnyExpr<'a>`) describing an expression
///   either as a view (structural node) or a borrowed encoded subtree.
/// - [`alloc_in`] may return an immutable reference to an already-allocated node (including
///   `self` if it is an arena reference already). [`alloc_in_mut`] always returns a unique mutable
///   reference suitable for later updates in builder-like scenarios.
pub trait ArenaAllocableExpr<'a> {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a>;

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a>;
}

/// Allocation context holding a `typed_arena` of expression nodes.
///
/// A context owns all nodes created within its lifetime. Drop the context when you no longer
/// need the arena-allocated views; you typically encode the final expression first.
pub struct ExprArenaCtx<'a> {
    arena: Arena<ArenaAnyExpr<'a>>,
}

impl<'a> ExprArenaCtx<'a> {
    /// Create a new, empty arena context.
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }

    /// Allocate a concrete arena expression into this context.
    ///
    /// Note: this simply forwards to `typed_arena::Arena::alloc`.
    pub fn alloc_expr(&'a self, expr: ArenaAnyExpr<'a>) -> &'a mut ArenaAnyExpr<'a> {
        self.arena.alloc(expr)
    }
}

impl<'a> Default for ExprArenaCtx<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// Arena-local expression node.
///
/// Two storage forms are supported:
/// - [`ArenaView`](ArenaAnyExpr::ArenaView): a structural node expressed as an [`ExprView`] over
///   arena references.
/// - [`ExprRef`](ArenaAnyExpr::ExprRef): a borrowed, already-encoded subtree (`AnyExprRef`). When
///   encoding, these are treated as leaves and copied into the target buffer via
///   [`TreeBuf::push_tree`](crate::encoding::tree::TreeBuf::push_tree).
#[derive(Clone)]
pub enum ArenaAnyExpr<'a> {
    ArenaView(ExprView<&'a ArenaAnyExpr<'a>, &'a ArenaAnyExpr<'a>, &'a ArenaAnyExpr<'a>>),
    ExprRef(AnyExprRef<'a>),
}

impl EncodableExpr for ArenaAnyExpr<'_> {
    /// Encode this arena expression into a compact buffer.
    ///
    /// Implementation detail: we perform an explicit iterative post-order traversal to avoid
    /// recursion and collect child node references before pushing each parent.
    fn encode_tree_step(&self, treebuf: &mut crate::encoding::tree::TreeBuf) -> TreeBufNodeRef {
        use ExprType::*;

        enum Frame<'a> {
            Enter(&'a ArenaAnyExpr<'a>),
            Exit(&'a ArenaAnyExpr<'a>),
        }

        let mut stack: SmallVec<[Frame<'_>; 16]> = SmallVec::new();
        let mut results: SmallVec<[TreeBufNodeRef; 16]> = SmallVec::new();

        stack.push(Frame::Enter(self));

        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Enter(node) => match node {
                    ArenaAnyExpr::ExprRef(r) => {
                        // Treat borrowed encoded subtrees as leaves by copying them in
                        results.push(r.encode_tree_step(treebuf));
                    }
                    ArenaAnyExpr::ArenaView(view) => {
                        // Post-order: visit children, then build this node
                        stack.push(Frame::Exit(node));
                        match view {
                            ExprView::Variable(_)
                            | ExprView::Bool
                            | ExprView::Omega
                            | ExprView::True
                            | ExprView::False
                            | ExprView::Never => {}
                            ExprView::Not(e) | ExprView::Powerset(e) => {
                                stack.push(Frame::Enter(e));
                            }
                            ExprView::And(a, b)
                            | ExprView::Or(a, b)
                            | ExprView::Implies(a, b)
                            | ExprView::Iff(a, b)
                            | ExprView::Equal(a, b)
                            | ExprView::Tuple(a, b) => {
                                stack.push(Frame::Enter(b));
                                stack.push(Frame::Enter(a));
                            }
                            ExprView::Lambda { arg, body }
                            | ExprView::Call {
                                func: arg,
                                arg: body,
                            } => {
                                // Maintain field semantics explicitly
                                stack.push(Frame::Enter(body));
                                stack.push(Frame::Enter(arg));
                            }
                            ExprView::Forall { dtype, inner, .. }
                            | ExprView::Exists { dtype, inner, .. } => {
                                stack.push(Frame::Enter(inner));
                                stack.push(Frame::Enter(dtype));
                            }
                            ExprView::If {
                                condition,
                                then_branch,
                                else_branch,
                            } => {
                                stack.push(Frame::Enter(else_branch));
                                stack.push(Frame::Enter(then_branch));
                                stack.push(Frame::Enter(condition));
                            }
                        }
                    }
                },
                Frame::Exit(node) => match node {
                    ArenaAnyExpr::ExprRef(_) => {
                        // Already handled as leaf
                    }
                    ArenaAnyExpr::ArenaView(view) => {
                        let noderef = match view {
                            ExprView::Variable(v) => {
                                treebuf.push_node(Variable as u8, Some(v.raw()), &[])
                            }
                            ExprView::Bool => treebuf.push_node(Bool as u8, None, &[]),
                            ExprView::Omega => treebuf.push_node(Omega as u8, None, &[]),
                            ExprView::True => treebuf.push_node(True as u8, None, &[]),
                            ExprView::False => treebuf.push_node(False as u8, None, &[]),
                            ExprView::Never => treebuf.push_node(Never as u8, None, &[]),
                            ExprView::Not(_) => {
                                let c = results.pop().unwrap();
                                treebuf.push_node(Not as u8, None, &[c])
                            }
                            ExprView::Powerset(_) => {
                                let c = results.pop().unwrap();
                                treebuf.push_node(Powerset as u8, None, &[c])
                            }
                            ExprView::And(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(And as u8, None, &[l, r])
                            }
                            ExprView::Or(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Or as u8, None, &[l, r])
                            }
                            ExprView::Implies(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Implies as u8, None, &[l, r])
                            }
                            ExprView::Iff(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Iff as u8, None, &[l, r])
                            }
                            ExprView::Equal(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Equal as u8, None, &[l, r])
                            }
                            ExprView::Lambda { .. } => {
                                let body = results.pop().unwrap();
                                let arg = results.pop().unwrap();
                                treebuf.push_node(Lambda as u8, None, &[arg, body])
                            }
                            ExprView::Call { .. } => {
                                let arg = results.pop().unwrap();
                                let func = results.pop().unwrap();
                                treebuf.push_node(Call as u8, None, &[func, arg])
                            }
                            ExprView::Tuple(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Tuple as u8, None, &[l, r])
                            }
                            ExprView::Forall { variable, .. } => {
                                let inner = results.pop().unwrap();
                                let dtype = results.pop().unwrap();
                                treebuf.push_node(
                                    Forall as u8,
                                    Some(variable.raw()),
                                    &[dtype, inner],
                                )
                            }
                            ExprView::Exists { variable, .. } => {
                                let inner = results.pop().unwrap();
                                let dtype = results.pop().unwrap();
                                treebuf.push_node(
                                    Exists as u8,
                                    Some(variable.raw()),
                                    &[dtype, inner],
                                )
                            }
                            ExprView::If { .. } => {
                                let else_b = results.pop().unwrap();
                                let then_b = results.pop().unwrap();
                                let cond = results.pop().unwrap();
                                treebuf.push_node(If as u8, None, &[cond, then_b, else_b])
                            }
                        };
                        results.push(noderef);
                    }
                },
            }
        }

        debug_assert_eq!(results.len(), 1);
        results.pop().unwrap()
    }
}

impl<'a> Expr for ArenaAnyExpr<'a> {
    /// View this arena expression as a generic [`ExprView`], abstracting over the two storage forms
    /// (`ArenaView` and `ExprRef`).
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        match self {
            ArenaAnyExpr::ArenaView(view) => (*view).map_unary(|x, _| Either::Left(x)),
            ArenaAnyExpr::ExprRef(any_expr_ref) => {
                any_expr_ref.view_typed().map_unary(|x, _| Either::Right(x))
            }
        }
    }
}

impl<'a> ArenaAllocableExpr<'a> for &'a ArenaAnyExpr<'a> {
    /// If already an arena reference, return it directly (no re-allocation).
    fn alloc_in(&self, _ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        self
    }

    /// Clone the node into a fresh allocation inside the context.
    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        ctx.alloc_expr((*self).clone())
    }
}

/// Create an arena context and run a callback within it.
///
/// This is useful for quick one-off allocations when you don't need to keep the context around.
///
/// Example
/// ```
/// use hyformal::arena::{with_arena_ctx, ArenaAnyExpr};
/// use hyformal::expr::{Expr, view::ExprView};
///
/// with_arena_ctx(|ctx| {
///     let t = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::True));
///     let not_t = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(t)));
///     assert_eq!(not_t.view().type_(), hyformal::expr::variant::ExprType::Not);
/// });
/// ```
pub fn with_arena_ctx<F>(callback: F)
where
    F: for<'a> FnOnce(&'a ExprArenaCtx<'a>),
{
    let ctx = ExprArenaCtx {
        arena: Arena::new(),
    };
    callback(&ctx);
}
