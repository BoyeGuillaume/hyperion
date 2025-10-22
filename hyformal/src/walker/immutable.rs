use std::{cell::RefCell, ops::Deref};

use crate::{
    expr::{IntoRecursiveExpr, sealed::ImplRecursiveExpr, view::ExprView},
    walker::internal::{InternalWalkerHandle, InternalWalkerNodeHandle, WalkerStackType},
};

// Alias the frequently-used complex view type to satisfy clippy's type_complexity lint.
type WalkerExprView<'a, I, R> =
    ExprView<WalkerNodeHandle<'a, I, R>, WalkerNodeHandle<'a, I, R>, WalkerNodeHandle<'a, I, R>>;

/// Lightweight handle passed to the visitor, representing a child node plus scheduling control.
///
/// - `E` is the underlying expression reference type (here [`AnyExprRef`]).
/// - `I` is the user-defined input/state type threaded through the traversal.
///
/// You can deref or `as_ref()` this handle to access the underlying expression, or call
/// [`schedule_visit`](Self::schedule_visit) to enqueue this child for a later DFS visit with an
/// input of your choice.
pub struct WalkerNodeHandle<'a, I, R: ImplRecursiveExpr> {
    internal: InternalWalkerNodeHandle<'a, I, <R as ImplRecursiveExpr>::Handle>,
    elem: R,
}

impl<'a, I, R: ImplRecursiveExpr> WalkerNodeHandle<'a, I, R> {
    /// Schedule this child to be visited immediately (LIFO), i.e., depth-first.
    /// Useful for drilling down before exploring siblings.
    #[inline]
    pub fn schedule_immediate(&self, input: I) {
        self.internal.schedule_immediate(input);
    }

    /// Schedule this child to be visited later (FIFO), i.e., breadth-first.
    /// Useful for exploring siblings before going deeper.
    #[inline]
    pub fn schedule_deferred(&self, input: I) {
        self.internal.schedule_deferred(input);
    }

    /// Convenience alias: schedule this child for a DFS visit (immediate/LIFO).
    ///
    /// Shorthand used throughout examples and docs.
    #[inline]
    pub fn schedule_visit(&self, input: I) {
        self.schedule_immediate(input)
    }

    /// Stop visiting this subtree entirely. Equivalent to breaking out of a loop. Notice that
    /// if node are scheduled after this call, they will still be visited.
    #[inline]
    pub fn break_(&self) {
        self.internal.break_();
    }

    #[inline]
    pub fn r#break(&self) {
        self.break_();
    }
}

impl<'a, I, R: ImplRecursiveExpr> AsRef<R> for WalkerNodeHandle<'a, I, R> {
    fn as_ref(&self) -> &R {
        &self.elem
    }
}

impl<'a, I, R: ImplRecursiveExpr> Deref for WalkerNodeHandle<'a, I, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.elem
    }
}

/// Lightweight handle passed to the visitor, representing the current node.
///
/// - `E` is the underlying expression reference type (here [`AnyExprRef`]).
/// - `I` is the user-defined input/state type threaded through the traversal.
///
/// You can deref or `as_ref()` this handle to access the underlying expression. You
/// can also directly match on this handle to access its children as if you were using
/// [`ExprView`]. Additionally, you can call [`schedule_parent`](Self::schedule_parent_immediate) to
/// enqueue the parent node for a later re-visit.
///
pub struct WalkerHandle<'a, I, R: ImplRecursiveExpr> {
    internal: InternalWalkerHandle<'a, I, <R as ImplRecursiveExpr>::Handle>,
    view: &'a WalkerExprView<'a, I, R>,
}

impl<'a, I, R: ImplRecursiveExpr> WalkerHandle<'a, I, R> {
    /// Re-schedule the current node to be visited immediately.
    #[inline]
    pub fn schedule_self_immediate(&self, input: I) {
        self.internal.schedule_self_immediate(input);
    }

    /// Check if this node is the root of the expression tree.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.internal.is_root()
    }
}

impl<'a, I, R: ImplRecursiveExpr> AsRef<WalkerExprView<'a, I, R>> for WalkerHandle<'a, I, R> {
    #[inline]
    fn as_ref(
        &self,
    ) -> &ExprView<WalkerNodeHandle<'a, I, R>, WalkerNodeHandle<'a, I, R>, WalkerNodeHandle<'a, I, R>>
    {
        self.view
    }
}

impl<'a, I, R: ImplRecursiveExpr> Deref for WalkerHandle<'a, I, R> {
    type Target = WalkerExprView<'a, I, R>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/// Walk an encoded expression in a DFS, user-scheduled manner.
///
/// The visitor receives the current input/state `I` and an [`ExprView`] of `WalkerHandle`s to
/// children. Call [`WalkerHandle::schedule_visit`] on the children you want to traverse.
///
/// Determinism: children scheduled for a node are visited in ascending order of their
/// underlying buffer index; with DFS and a LIFO stack this yields a stable pre-order across runs.
pub fn walk<F, I, R: IntoRecursiveExpr>(expr: R, input: I, mut walker: F)
where
    F: FnMut(I, WalkerHandle<'_, I, R::RecursiveExpressionType>),
{
    let base_expr = expr.into_recursive();

    // Stack of (node_ref, parent, input)
    let stack = RefCell::new(WalkerStackType::<
        I,
        <R::RecursiveExpressionType as ImplRecursiveExpr>::Handle,
    >::new());

    // Add root node to the stack
    stack
        .borrow_mut()
        .push_front((base_expr.recursed_root(), None, input));

    // Traverse the tree
    loop {
        // Pop with a short-lived mutable borrow to avoid overlapping borrows
        let next = {
            let mut s = stack.borrow_mut();
            s.pop_front()
        };
        let Some((current_node, parent, input)) = next else {
            break;
        };

        // Extract the node from the reference
        let view = base_expr
            .recursed_view(current_node)
            .map_unary(|elem, _| WalkerNodeHandle {
                internal: InternalWalkerNodeHandle {
                    stack: &stack,
                    children_node: elem,
                    current_node,
                },
                elem: base_expr.recursed_handle_into(elem),
            });
        // let view = AnyExpr::_view(expr.tree, current_node).map_unary(|elem, _| WalkerNodeHandle {
        //     internal: InternalWalkerNodeHandle {
        //         stack: &stack,
        //         children_node: elem.node,
        //         current_node,
        //     },
        //     elem,
        // });

        // Apply the walker function
        walker(
            input,
            WalkerHandle {
                internal: InternalWalkerHandle {
                    stack: &stack,
                    parent,
                    current_node,
                },
                view: &view,
            },
        );
    }
}

/// Convenience when no input/state needs to be threaded.
#[inline]
pub fn walk_no_input<F, R: IntoRecursiveExpr>(expr: R, mut walker: F)
where
    F: FnMut(WalkerHandle<'_, (), R::RecursiveExpressionType>),
{
    walk(expr, (), |(), node| walker(node));
}
