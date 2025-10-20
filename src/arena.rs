use std::{cell::RefCell, ops::Deref};

use either::Either;
use smallvec::SmallVec;
use typed_arena::Arena;

use crate::{
    encoding::{EncodableExpr, tree::TreeBufNodeRef},
    expr::{AnyExprRef, Expr, variant::ExprType, view::ExprView},
    walker::walk,
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
/// - Implementors produce arena-lifetime values (`&'a RefCell<ArenaAnyExpr<'a>>`) describing an
///   expression either as a view (structural node) or a borrowed encoded subtree.
/// - [`alloc_in`] may return a reference to an already-allocated node (including `self` if it is
///   an arena reference already). Interior mutability is available through `RefCell` when needed.
pub trait ArenaAllocableExpr<'a> {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a RefCell<ArenaAnyExpr<'a>>;
}

/// Allocation context holding a `typed_arena` of expression nodes.
///
/// A context owns all nodes created within its lifetime. Drop the context when you no longer
/// need the arena-allocated views; you typically encode the final expression first.
pub struct ExprArenaCtx<'a> {
    arena: Arena<RefCell<ArenaAnyExpr<'a>>>,
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
    pub fn alloc_expr(&'a self, expr: ArenaAnyExpr<'a>) -> &'a RefCell<ArenaAnyExpr<'a>> {
        self.arena.alloc(RefCell::new(expr))
    }

    /// Create an arena node that references a pre-encoded external subtree.
    ///
    /// The referenced [`AnyExprRef`] is treated as a leaf during encoding of the arena tree and
    /// copied into the target buffer. This allows mixing owned/borrowed subtrees with arena-local
    /// structural nodes.
    pub fn reference_external(&'a self, expr: AnyExprRef<'a>) -> &'a RefCell<ArenaAnyExpr<'a>> {
        self.arena.alloc(RefCell::new(ArenaAnyExpr::ExprRef(expr)))
    }

    /// Deep copy an arena-backed expression within this context.
    ///
    /// Semantics
    /// - Performs an iterative post-order traversal of `expr` and allocates a structurally
    ///   identical tree of new nodes inside this arena.
    /// - Returns a reference to the newly-allocated root node.
    /// - Leaves that are borrowed encoded subtrees ([`AnyExprRef`]) are preserved as borrowed
    ///   leaves in the copy; when the arena tree is encoded later, such leaves are copied into
    ///   the destination buffer.
    ///
    /// Complexity
    /// - O(n) over the size of the input tree; allocation cost is amortized by the arena.
    ///
    /// Example
    /// ```
    /// use hyformal::arena::{ExprArenaCtx, ArenaAnyExpr};
    /// use hyformal::expr::view::ExprView;
    /// use hyformal::expr::{Expr, variant::ExprType};
    ///
    /// let ctx = ExprArenaCtx::new();
    /// let t = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::True));
    /// let not_t = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(t)));
    /// let copy = ctx.deep_copy(not_t);
    ///
    /// // Both encode to structurally equal buffers
    /// let e1 = not_t.encode();
    /// let e2 = copy.encode();
    /// assert_eq!(e1.as_ref().type_(), ExprType::Not);
    /// assert!(e1 == e2);
    /// ```
    pub fn deep_copy(&'a self, expr: &RefCell<ArenaAnyExpr<'a>>) -> &'a RefCell<ArenaAnyExpr<'a>> {
        enum Frame<'a_, 'b> {
            Enter(&'b RefCell<ArenaAnyExpr<'a_>>),
            Exit(&'b RefCell<ArenaAnyExpr<'a_>>),
        }

        // Post-order iterative traversal to rebuild the tree in this arena.
        let mut stack: SmallVec<[Frame<'a, '_>; 16]> = SmallVec::new();
        // Hold newly allocated children while bubbling up; use shared refs for assembling views.
        let mut results: SmallVec<[&'a RefCell<ArenaAnyExpr<'a>>; 16]> = SmallVec::new();
        // Track the allocation corresponding to the requested root to return a ref.
        let mut root_alloc: Option<&'a RefCell<ArenaAnyExpr<'a>>> = None;

        stack.push(Frame::Enter(expr));

        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Enter(node) => match *node.borrow() {
                    // Treat borrowed encoded subtrees as leaves; handle on Exit.
                    ArenaAnyExpr::ExprRef(_) => {
                        stack.push(Frame::Exit(node));
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
                Frame::Exit(node) => match *node.borrow() {
                    ArenaAnyExpr::ExprRef(r) => {
                        // Leaf: clone the external reference as a leaf in the copy
                        let alloc = self.alloc_expr(ArenaAnyExpr::ExprRef(r));
                        if std::ptr::eq(node, expr) {
                            root_alloc = Some(alloc);
                        } else {
                            results.push(alloc);
                        }
                    }
                    ArenaAnyExpr::ArenaView(view) => {
                        let new_view = match view {
                            ExprView::Variable(v) => ExprView::Variable(v),
                            ExprView::Bool => ExprView::Bool,
                            ExprView::Omega => ExprView::Omega,
                            ExprView::True => ExprView::True,
                            ExprView::False => ExprView::False,
                            ExprView::Never => ExprView::Never,
                            ExprView::Not(_) => {
                                let c = results.pop().unwrap();
                                ExprView::Not(c)
                            }
                            ExprView::Powerset(_) => {
                                let c = results.pop().unwrap();
                                ExprView::Powerset(c)
                            }
                            ExprView::And(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                ExprView::And(l, r)
                            }
                            ExprView::Or(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                ExprView::Or(l, r)
                            }
                            ExprView::Implies(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                ExprView::Implies(l, r)
                            }
                            ExprView::Iff(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                ExprView::Iff(l, r)
                            }
                            ExprView::Equal(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                ExprView::Equal(l, r)
                            }
                            ExprView::Lambda { .. } => {
                                let body = results.pop().unwrap();
                                let arg = results.pop().unwrap();
                                ExprView::Lambda { arg, body }
                            }
                            ExprView::Call { .. } => {
                                let arg = results.pop().unwrap();
                                let func = results.pop().unwrap();
                                ExprView::Call { func, arg }
                            }
                            ExprView::Tuple(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                ExprView::Tuple(l, r)
                            }
                            ExprView::Forall { variable, .. } => {
                                let inner = results.pop().unwrap();
                                let dtype = results.pop().unwrap();
                                ExprView::Forall {
                                    variable,
                                    dtype,
                                    inner,
                                }
                            }
                            ExprView::Exists { variable, .. } => {
                                let inner = results.pop().unwrap();
                                let dtype = results.pop().unwrap();
                                ExprView::Exists {
                                    variable,
                                    dtype,
                                    inner,
                                }
                            }
                            ExprView::If { .. } => {
                                let else_b = results.pop().unwrap();
                                let then_b = results.pop().unwrap();
                                let cond = results.pop().unwrap();
                                ExprView::If {
                                    condition: cond,
                                    then_branch: then_b,
                                    else_branch: else_b,
                                }
                            }
                        };

                        let alloc = self.alloc_expr(ArenaAnyExpr::ArenaView(new_view));
                        if std::ptr::eq(node, expr) {
                            root_alloc = Some(alloc);
                        } else {
                            results.push(alloc);
                        }
                    }
                },
            }
        }

        debug_assert!(results.is_empty());
        root_alloc.expect("deep_copy should produce a root allocation")
    }

    /// Import and deeply copy a borrowed, already-encoded expression into this arena.
    ///
    /// Purpose
    /// - Reconstructs a tree of arena-local nodes from an [`AnyExprRef`], so the result can be
    ///   mixed with other arena-allocated views and further transformed without touching the
    ///   original buffer.
    ///
    /// Semantics
    /// - Traverses `expr_ref` iteratively and rebuilds an equivalent tree as [`ArenaView`] nodes
    ///   allocated inside this context.
    /// - Returns a reference to the freshly allocated root (`&RefCell<ArenaAnyExpr>`).
    /// - The copy is independent from the source; subsequent mutations to the arena tree do not
    ///   affect `expr_ref`.
    ///
    /// Notes
    /// - If you already have an arena root and want to copy it within the same context, see
    ///   [`ExprArenaCtx::deep_copy`]. If you merely want to reference an external subtree as a
    ///   leaf (without expanding it into arena nodes), use [`ExprArenaCtx::reference_external`].
    ///
    /// Complexity
    /// - O(n) allocations and time where n is the number of nodes in `expr_ref`.
    ///
    /// Example: copy a borrowed encoded tree and extend it in the arena
    /// ```
    /// use hyformal::arena::{ExprArenaCtx, ArenaAnyExpr};
    /// use hyformal::expr::defs::*;
    /// use hyformal::expr::{Expr, variant::ExprType};
    ///
    /// // Build an owned, encoded expression first
    /// let owned = Not { inner: And { lhs: True, rhs: False } }.encode();
    /// let borrowed = owned.as_ref();
    ///
    /// // Bring it into an arena as a fully materialized tree
    /// let ctx = ExprArenaCtx::new();
    /// let arena_root = ctx.deep_copy_ref(borrowed);
    ///
    /// // We can now build more nodes around it in the arena
    /// let wrapped = ctx.alloc_expr(ArenaAnyExpr::ArenaView(hyformal::expr::view::ExprView::Not(arena_root)));
    ///
    /// // Both the original borrowed tree and the arena copy encode to the same structure
    /// assert!(owned == arena_root.encode());
    /// assert_eq!(wrapped.view().type_(), ExprType::Not);
    /// ```
    pub fn deep_copy_ref(&'a self, expr_ref: AnyExprRef<'a>) -> &'a RefCell<ArenaAnyExpr<'a>> {
        let mut latest_stack: SmallVec<[&'a RefCell<ArenaAnyExpr<'a>>; 16]> = SmallVec::new();

        #[derive(Debug)]
        pub enum WalkState {
            Enter,
            Exit,
        }

        walk(expr_ref, WalkState::Enter, |state, ctx| {
            match state {
                WalkState::Enter => {
                    // Reschedule the current node after processing its children
                    ctx.schedule_self_immediate(WalkState::Exit);

                    // First we generate the left and right children (if any), then
                    // we backtrack and revisit the current node once both children are ready.
                    ctx.deref().for_each_unary(|elem, _| {
                        // Store the child in a temporary location for later retrieval
                        elem.schedule_immediate(WalkState::Enter);
                    });
                }
                WalkState::Exit => {
                    // Reconstruct the current node with allocated children
                    let new_node = ctx
                        .deref()
                        .as_ref()
                        .map_unary(|_, _| latest_stack.pop().unwrap());
                    let new_node = self.alloc_expr(ArenaAnyExpr::ArenaView(new_node));
                    latest_stack.push(new_node);
                }
            }
        });

        latest_stack.pop().unwrap()
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
    ArenaView(
        ExprView<
            &'a RefCell<ArenaAnyExpr<'a>>,
            &'a RefCell<ArenaAnyExpr<'a>>,
            &'a RefCell<ArenaAnyExpr<'a>>,
        >,
    ),
    ExprRef(AnyExprRef<'a>),
}

impl<'a> EncodableExpr for RefCell<ArenaAnyExpr<'a>> {
    /// Encode this arena expression into a compact buffer.
    ///
    /// Implementation detail: we perform an explicit iterative post-order traversal to avoid
    /// recursion and collect child node references before pushing each parent.
    fn encode_tree_step(&self, treebuf: &mut crate::encoding::tree::TreeBuf) -> TreeBufNodeRef {
        use ExprType::*;

        enum Frame<'a, 'b> {
            Enter(&'b RefCell<ArenaAnyExpr<'a>>),
            Exit(&'b RefCell<ArenaAnyExpr<'a>>),
        }

        let mut stack: SmallVec<[Frame; 16]> = SmallVec::new();
        let mut results: SmallVec<[TreeBufNodeRef; 16]> = SmallVec::new();
        stack.push(Frame::Enter(self));

        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Enter(node) => match *node.borrow() {
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
                Frame::Exit(node) => match *node.borrow() {
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

impl<'a> Expr for RefCell<ArenaAnyExpr<'a>> {
    /// View this arena expression as a generic [`ExprView`], abstracting over the two storage forms
    /// (`ArenaView` and `ExprRef`).
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        match *self.borrow() {
            ArenaAnyExpr::ArenaView(view) => view.map_unary(|x, _| Either::Left(x)),
            ArenaAnyExpr::ExprRef(any_expr_ref) => {
                any_expr_ref.view_typed().map_unary(|x, _| Either::Right(x))
            }
        }
    }
}

impl<'a> ArenaAllocableExpr<'a> for &'a RefCell<ArenaAnyExpr<'a>> {
    /// If already an arena reference, return it directly (no re-allocation).
    fn alloc_in(&self, _ctx: &'a ExprArenaCtx<'a>) -> &'a RefCell<ArenaAnyExpr<'a>> {
        self
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
