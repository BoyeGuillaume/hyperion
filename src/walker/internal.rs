//! Internal primitives powering the public walker API.
//!
//! These types are intentionally kept in a separate module to avoid exposing
//! scheduling internals in the public surface. They provide the minimal handles
//! used by the public [`WalkerHandle`](super::immutable::WalkerHandle) and
//! [`WalkerNodeHandle`](super::immutable::WalkerNodeHandle) wrappers.
use std::{cell::RefCell, collections::VecDeque};

use crate::encoding::tree::{TreeBuf, TreeBufNodeRef};

/// Internal stack type: triples of (children_node, current_node, input).
///
/// - `children_node`: the node whose children will be scheduled next.
/// - `current_node`: the node currently being visited (its parent for scheduled children).
/// - `input`: user-provided state threaded through the traversal.
pub(super) type WalkerStackType<I> = VecDeque<(TreeBufNodeRef, TreeBufNodeRef, I)>;

/// Handle for scheduling a child visit from within a visitor.
///
/// This is the low-level counterpart used by the public `WalkerNodeHandle` wrapper.
pub(super) struct InternalWalkerNodeHandle<'a, I> {
    pub(super) stack: &'a RefCell<WalkerStackType<I>>,
    pub(super) children_node: TreeBufNodeRef,
    pub(super) current_node: TreeBufNodeRef,
}

impl<'a, I> InternalWalkerNodeHandle<'a, I> {
    /// Schedule the child to be visited immediately (LIFO/DFS).
    #[inline]
    pub fn schedule_immediate(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_front((self.children_node, self.current_node, input));
    }

    /// Schedule the child to be visited later (FIFO/BFS).
    #[inline]
    pub fn schedule_deferred(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_back((self.children_node, self.current_node, input));
    }
}

/// Handle for scheduling the parent or re-scheduling the current node.
///
/// Used internally by the public `WalkerHandle` wrapper to offer ergonomic methods.
pub(super) struct InternalWalkerHandle<'a, I> {
    pub(super) stack: &'a RefCell<WalkerStackType<I>>,
    pub(super) parent: TreeBufNodeRef,
    pub(super) current_node: TreeBufNodeRef,
}

impl<'a, I> InternalWalkerHandle<'a, I> {
    /// Schedule the parent of the current node for an immediate visit.
    ///
    /// Panics in debug if called on the root node (which has no parent).
    #[inline]
    pub fn schedule_parent_immediate(&self, input: I) {
        assert!(
            self.parent != TreeBuf::INVALID_NODE_REF,
            "Cannot schedule parent of root node"
        );

        self.stack
            .borrow_mut()
            .push_front((self.parent, self.current_node, input));
    }

    /// Return `true` if the current node is the root of the tree.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent == TreeBuf::INVALID_NODE_REF
    }

    /// Re-schedule the current node for an immediate re-visit.
    pub(crate) fn schedule_self_immediate(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_front((self.parent, self.current_node, input));
    }
}
