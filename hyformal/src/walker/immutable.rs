use std::{cell::RefCell, ops::Deref};

use crate::{
    encoding::tree::TreeBuf,
    expr::{AnyExpr, AnyExprRef, view::ExprView},
    walker::internal::{InternalWalkerHandle, InternalWalkerNodeHandle, WalkerStackType},
};

/// Lightweight handle passed to the visitor, representing a child node plus scheduling control.
///
/// - `E` is the underlying expression reference type (here [`AnyExprRef`]).
/// - `I` is the user-defined input/state type threaded through the traversal.
///
/// You can deref or `as_ref()` this handle to access the underlying expression, or call
/// [`schedule_visit`](Self::schedule_visit) to enqueue this child for a later DFS visit with an
/// input of your choice.
pub struct WalkerNodeHandle<'a, I> {
    internal: InternalWalkerNodeHandle<'a, I>,
    elem: AnyExprRef<'a>,
}

impl<'a, I> WalkerNodeHandle<'a, I> {
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
}

impl<'a, I> AsRef<AnyExprRef<'a>> for WalkerNodeHandle<'a, I> {
    fn as_ref(&self) -> &AnyExprRef<'a> {
        &self.elem
    }
}

impl<'a, I> Deref for WalkerNodeHandle<'a, I> {
    type Target = AnyExprRef<'a>;

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
pub struct WalkerHandle<'a, I> {
    internal: InternalWalkerHandle<'a, I>,
    view: &'a ExprView<WalkerNodeHandle<'a, I>, WalkerNodeHandle<'a, I>, WalkerNodeHandle<'a, I>>,
}

impl<'a, I> WalkerHandle<'a, I> {
    /// Schedule the parent node to be visited immediately. Notice that if this is called within the
    /// iteration of a child, the parent will be revisited immediately after the current node's processing.
    ///
    /// You can make use of the input/state [`I`] to only schedule parent after the visitation of the last
    /// child to achieve a post-order traversal. Callers must ensure that this is not invoked on the root node.
    /// You can check if this node is the root with [`is_root`](Self::is_root).
    #[inline]
    pub fn schedule_parent_immediate(&self, input: I) {
        self.internal.schedule_parent_immediate(input);
    }

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

impl<'a, I>
    AsRef<ExprView<WalkerNodeHandle<'a, I>, WalkerNodeHandle<'a, I>, WalkerNodeHandle<'a, I>>>
    for WalkerHandle<'a, I>
{
    #[inline]
    fn as_ref(
        &self,
    ) -> &ExprView<WalkerNodeHandle<'a, I>, WalkerNodeHandle<'a, I>, WalkerNodeHandle<'a, I>> {
        self.view
    }
}

impl<'a, I> Deref for WalkerHandle<'a, I> {
    type Target =
        ExprView<WalkerNodeHandle<'a, I>, WalkerNodeHandle<'a, I>, WalkerNodeHandle<'a, I>>;

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
pub fn walk<F, I>(expr: AnyExprRef, input: I, mut walker: F)
where
    F: FnMut(I, WalkerHandle<'_, I>),
{
    // Stack of (node_ref, parent, input)
    let stack = RefCell::new(WalkerStackType::<I>::new());

    // Add root node to the stack
    stack
        .borrow_mut()
        .push_front((expr.node, TreeBuf::INVALID_NODE_REF, input));

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
        let view = AnyExpr::_view(expr.tree, current_node).map_unary(|elem, _| WalkerNodeHandle {
            internal: InternalWalkerNodeHandle {
                stack: &stack,
                children_node: elem.node,
                current_node,
            },
            elem,
        });

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
pub fn walk_no_input<F>(expr: AnyExprRef, mut walker: F)
where
    F: FnMut(WalkerHandle<'_, ()>),
{
    walk(expr, (), |(), node| walker(node));
}
