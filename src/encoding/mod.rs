//! Internal encoding utilities shared by dtype, expr, and prop.
//!
//! Users typically interact with higher-level modules; this module documents
//! the layout and performance characteristics for completeness.
use crate::encoding::tree::TreeBufNodeRef;
pub mod tree;

/// Trait for types that can be encoded as a tree
pub trait EncodableExpr: Clone {
    fn encode_tree_step(self, tree: &mut tree::TreeBuf) -> TreeBufNodeRef;

    #[inline]
    fn encode_tree(self, tree: &mut tree::TreeBuf) {
        let noderef = self.encode_tree_step(tree);
        tree.set_root(noderef);
        tree.consolite_if_needed();
    }
}

impl<'a, T: EncodableExpr> EncodableExpr for &'a T {
    #[inline]
    fn encode_tree_step(self, tree: &mut tree::TreeBuf) -> TreeBufNodeRef {
        (*self).clone().encode_tree_step(tree)
    }
}
