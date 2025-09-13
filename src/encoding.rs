use smallvec::SmallVec;
pub mod integer;

/// A small, stack-allocated-first buffer used by the encoder.
///
/// Backed by `smallvec`, this stores up to 32 bytes inline before spilling to the heap.
pub type DynBuf = SmallVec<[u8; 32]>;

pub trait RawEncodable {
    fn encode_raw(&self, buf: &mut DynBuf);
}
