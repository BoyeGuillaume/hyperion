//! Internal encoding utilities shared by dtype, expr, and prop.
//!
//! Users typically interact with higher-level modules; this module documents
//! the layout and performance characteristics for completeness.
use smallvec::SmallVec;
pub mod integer;
pub mod magic;

/// A small, stack-allocated-first buffer used by the encoder.
///
/// Backed by `smallvec`, this stores up to 32 bytes inline before spilling to the heap.
pub type DynBuf = SmallVec<[u8; 32]>;

/// Trait for types that can append their raw encoding into a buffer.
pub trait RawEncodable {
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64;

    fn encode_dynbuf(&self, buf: &mut DynBuf) {
        self.encode_raw(&mut |b| buf.extend_from_slice(b));
    }

    fn encoded_size(&self) -> u64;
}

impl<T: RawEncodable> RawEncodable for &T {
    #[inline]
    fn encode_raw<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        (*self).encode_raw(f)
    }

    #[inline]
    fn encoded_size(&self) -> u64 {
        (*self).encoded_size()
    }
}
