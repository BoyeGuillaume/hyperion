//! Internal encoding utilities shared by the unified expression system.
//!
//! You normally work with higher-level builders and views in `expr::*`.
//! This module explains the shape and performance profile of the underlying
//! zero-copy tree buffer so you can reason about costs when encoding/decoding.
use either::Either;

use crate::encoding::tree::TreeBufNodeRef;
pub mod tree;

/// Trait for values that can be encoded into the compact tree buffer.
///
/// Role
/// - Provides a single-step method [`encode_tree_step`] used by the encoder to push a node
///   (and recursively its children) into a [`tree::TreeBuf`].
/// - The blanket impl for references allows you to pass `&T` where `T: EncodableExpr` without
///   moving the value.
///
/// Performance
/// - Each call encodes exactly one node and returns a handle [`TreeBufNodeRef`].
/// - Amortized O(1) push into the buffer; overall encoding is O(n) in the number of nodes.
/// - The buffer may occasionally consolidate to reclaim wasted space; see
///   [`tree::TreeBuf::consolidate`].
pub trait EncodableExpr {
    /// Encode a single node into `tree`, returning its node reference.
    ///
    /// Contract
    /// - Inputs: `self` value to encode, mutable target [`tree::TreeBuf`].
    /// - Output: [`TreeBufNodeRef`] pointing to the encoded node.
    /// - Error modes: This API does not return errors; internal debug assertions guard
    ///   invariants in debug builds.
    ///
    /// Complexity
    /// - O(1) for the single node push, not counting recursive child encodes you may perform.
    fn encode_tree_step(&self, tree: &mut tree::TreeBuf) -> TreeBufNodeRef;

    /// Encode `self` as a whole tree and set it as the root of `tree`.
    ///
    /// Convenience wrapper around [`encode_tree_step`]; after pushing the node it marks it as
    /// the root and triggers consolidation if beneficial.
    ///
    /// Complexity
    /// - O(n) in the number of nodes encoded. May trigger an occasional consolidation which is
    ///   also linear in the current buffer length.
    #[inline]
    fn encode_tree(&self, tree: &mut tree::TreeBuf) {
        let noderef = self.encode_tree_step(tree);
        tree.set_root(noderef);
    }
}

impl<T: EncodableExpr> EncodableExpr for &T {
    #[inline]
    fn encode_tree_step(&self, tree: &mut tree::TreeBuf) -> TreeBufNodeRef {
        (*self).encode_tree_step(tree)
    }
}

impl<L: EncodableExpr, R: EncodableExpr> EncodableExpr for Either<L, R> {
    #[inline]
    fn encode_tree_step(&self, tree: &mut tree::TreeBuf) -> TreeBufNodeRef {
        match self {
            Either::Left(l) => l.encode_tree_step(tree),
            Either::Right(r) => r.encode_tree_step(tree),
        }
    }
}
