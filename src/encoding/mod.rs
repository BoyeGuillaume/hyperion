//! Internal encoding utilities shared by dtype, expr, and prop.
//!
//! Users typically interact with higher-level modules; this module documents
//! the layout and performance characteristics for completeness.
use smallvec::SmallVec;

use crate::encoding::tree::TreeBufNodeRef;
pub mod integer;
pub mod legacy_magic;
pub mod tree;

/// A small, stack-allocated-first buffer used by the encoder.
///
/// Backed by `smallvec`, this stores up to 32 bytes inline before spilling to the heap.
pub type LegacyDynBuf = SmallVec<[u8; 32]>;

/// Trait for types that can be encoded as a tree
pub trait EncodableExpr: Clone {
    fn encode_tree_step(self, tree: &mut tree::TreeBuf) -> TreeBufNodeRef;

    fn encode_tree(self, tree: &mut tree::TreeBuf) {
        let noderef = self.encode_tree_step(tree);
        tree.update_root_node(noderef);
        tree.consolite_if_needed();
    }
}

/// Trait for types that can append their raw encoding into a buffer.
pub trait LegacyRawEncodable {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64;

    fn encode_dynbuf(&self, buf: &mut LegacyDynBuf) {
        self.encode_raw(&mut |b| buf.extend_from_slice(b));
    }

    fn encoded_size(&self) -> u64;
}

impl<T: LegacyRawEncodable> LegacyRawEncodable for &T {
    #[inline]
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        (*self).encode_raw(f)
    }

    #[inline]
    fn encoded_size(&self) -> u64 {
        (*self).encoded_size()
    }
}
