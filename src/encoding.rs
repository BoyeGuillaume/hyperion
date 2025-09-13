use smallvec::SmallVec;
pub mod integer;
pub mod magic;

/// A small, stack-allocated-first buffer used by the encoder.
///
/// Backed by `smallvec`, this stores up to 32 bytes inline before spilling to the heap.
pub type DynBuf = SmallVec<[u8; 32]>;

pub trait RawEncodable {
    fn encode_raw(&self, buf: &mut DynBuf);
}

#[inline]
pub(crate) fn push_len(len: usize, buf: &mut DynBuf) {
    integer::encode_u64(len as u64, buf);
}

impl<T: RawEncodable> RawEncodable for &T {
    #[inline]
    fn encode_raw(&self, buf: &mut DynBuf) {
        (*self).encode_raw(buf)
    }
}
