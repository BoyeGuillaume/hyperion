//! Internal primitives powering the public walker API.
//!
//! These types are intentionally kept in a separate module to avoid exposing
//! scheduling internals in the public surface. They provide the minimal handles
//! used by the public [`WalkerHandle`](super::immutable::WalkerHandle) and
//! [`WalkerNodeHandle`](super::immutable::WalkerNodeHandle) wrappers.
use std::{cell::RefCell, collections::VecDeque};

/// Internal stack type: triples of (children_node, current_node, input).
///
/// - `children_node`: the node whose children will be scheduled next.
/// - `current_node`: the node currently being visited (its parent for scheduled children).
/// - `input`: user-provided state threaded through the traversal.
pub(super) type WalkerStackType<I, H> = VecDeque<(H, Option<H>, I)>;

/// Handle for scheduling a child visit from within a visitor.
///
/// This is the low-level counterpart used by the public `WalkerNodeHandle` wrapper.
pub(super) struct InternalWalkerNodeHandle<'a, I, H: Sized + Copy> {
    pub(super) stack: &'a RefCell<WalkerStackType<I, H>>,
    pub(super) children_node: H,
    pub(super) current_node: H,
}

impl<'a, I, H: Sized + Copy> InternalWalkerNodeHandle<'a, I, H> {
    /// Schedule the child to be visited immediately (LIFO/DFS).
    #[inline]
    pub fn schedule_immediate(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_front((self.children_node, Some(self.current_node), input));
    }

    /// Schedule the child to be visited later (FIFO/BFS).
    #[inline]
    pub fn schedule_deferred(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_back((self.children_node, Some(self.current_node), input));
    }

    /// Stop visiting this subtree entirely. Equivalent to breaking out of a loop. Notice that
    /// if node are scheduled after this call, they will still be visited.
    #[inline]
    pub fn break_(&self) {
        self.stack.borrow_mut().clear();
    }
}

/// Handle for scheduling the parent or re-scheduling the current node.
///
/// Used internally by the public `WalkerHandle` wrapper to offer ergonomic methods.
pub(super) struct InternalWalkerHandle<'a, I, H: Sized + Copy> {
    pub(super) stack: &'a RefCell<WalkerStackType<I, H>>,
    pub(super) parent: Option<H>,
    pub(super) current_node: H,
}

impl<'a, I, H: Sized + Copy> InternalWalkerHandle<'a, I, H> {
    /// Return `true` if the current node is the root of the tree.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Re-schedule the current node for an immediate re-visit.
    pub(crate) fn schedule_self_immediate(&self, input: I) {
        // Revisit the same current node again; keep the same parent relation.
        self.stack
            .borrow_mut()
            .push_front((self.current_node, self.parent, input));
    }
}
