use std::{cell::RefCell, collections::VecDeque};

use crate::encoding::tree::{TreeBuf, TreeBufNodeRef};

pub(super) type WalkerStackType<I> = VecDeque<(TreeBufNodeRef, TreeBufNodeRef, I)>;

pub(super) struct InternalWalkerNodeHandle<'a, I> {
    pub(super) stack: &'a RefCell<WalkerStackType<I>>,
    pub(super) children_node: TreeBufNodeRef,
    pub(super) current_node: TreeBufNodeRef,
}

impl<'a, I> InternalWalkerNodeHandle<'a, I> {
    #[inline]
    pub fn schedule_immediate(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_front((self.children_node, self.current_node, input));
    }

    #[inline]
    pub fn schedule_deferred(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_back((self.children_node, self.current_node, input));
    }
}

pub(super) struct InternalWalkerHandle<'a, I> {
    pub(super) stack: &'a RefCell<WalkerStackType<I>>,
    pub(super) parent: TreeBufNodeRef,
    pub(super) current_node: TreeBufNodeRef,
}

impl<'a, I> InternalWalkerHandle<'a, I> {
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

    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent == TreeBuf::INVALID_NODE_REF
    }

    pub(crate) fn schedule_self_immediate(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_front((self.parent, self.current_node, input));
    }
}
