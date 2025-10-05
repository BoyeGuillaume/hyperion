//! Varint encoding helpers for compact u64 representation.
//!
//! These are internal utilities, but are documented to clarify size and performance
//! characteristics of the on-wire format used by this crate.

/// Encode an unsigned 64-bit integer into `buf` using a compact base-128 scheme.
///
/// Format:
/// - Split the value into 7-bit chunks (base-128 digits), least-significant first.
/// - Push the first (least significant) chunk with MSB = 0.
/// - Push each subsequent chunk with MSB = 1.
///
/// This pairs with [`decode_u64`], which decodes from the end of a byte slice
/// (i.e., values are expected to be appended sequentially and decoded in reverse).
///
/// Notes:
/// - Value 0 encodes to a single byte `0x00`.
/// - The encoding for 128 is `[0x00, 0x81]` (digits 0 and 1, with continuation on the latter).
///
/// Example (encoding only; doctest ignored because the module is crate-private):
/// ```ignore
/// use hyformal::encoding::{encode_u64, DynBuf};
/// let mut buf = DynBuf::new();
/// encode_u64(300, |b| buf.extend_from_slice(b));
/// assert_eq!(&buf[..], &[0x2c, 0x82]);
/// ```
pub fn encode_u64<F: FnMut(&[u8])>(mut value: u64, encoder: &mut F) -> u64 {
    // Push least-significant 7 bits with MSB cleared.
    let mut byte = (value & 0x7F) as u8;
    let mut size = 1;
    value >>= 7;
    encoder(&[byte]);

    // Push remaining chunks with MSB set to indicate continuation when decoding from the end.
    while value > 0 {
        byte = ((value & 0x7F) as u8) | 0x80;
        encoder(&[byte]);
        value >>= 7;
        size += 1;
    }

    size
}

/// Decode one unsigned 64-bit integer from the end of the given slice.
///
/// Behavior:
/// - Reads bytes from the end of `buf` (last element first).
/// - Accumulates 7-bit chunks until a byte with MSB = 0 is found, which terminates the value.
/// - On success, returns `Some(value)` and updates `buf` to the remaining prefix.
/// - If the slice is exhausted before finding a terminating byte, returns `None` and `buf` is left empty.
///
/// Example (decoding only; doctest ignored because the module is crate-private):
/// ```ignore
/// use hyformal::encoding::decode_u64;
/// let mut slice: &[u8] = &[0x2c, 0x82]; // 300
/// let v = decode_u64(&mut slice);
/// assert_eq!(v, Some(300));
/// assert!(slice.is_empty());
/// ```
pub fn decode_u64(buf: &mut &[u8]) -> Option<u64> {
    let mut value: u64 = 0;

    loop {
        // Check the last byte of the buffer
        let byte = *buf.last()?;
        *buf = &buf[..buf.len() - 1];
        value = (value << 7) | (byte as u64 & 0x7F);

        if byte & 0x80 == 0 {
            break Some(value);
        }
    }
}

/// Compute the encoded size in bytes of a u64 value using the varint scheme.
///
/// This is useful for preallocating buffers or estimating storage requirements
///
pub fn encoded_size_u64(value: u64) -> u64 {
    if value == 0 {
        return 1;
    }
    // Number of significant bits is 64 - leading_zeros; convert to u64.
    let sig_bits = (64 - value.leading_zeros()) as u64;
    // Each byte encodes 7 bits; ceil_div(sig_bits, 7)
    (sig_bits + 6) / 7
}

#[cfg(test)]
mod tests {
    use crate::encoding::DynBuf;

    use super::*;

    #[test]
    fn roundtrip_small_values() {
        let values = [
            0_u64, 1, 2, 3, 10, 42, 63, 64, 65, 100, 127, 128, 129, 255, 256, 300,
        ];
        for &v in &values {
            let mut buf = DynBuf::new();
            encode_u64(v, &mut |b| buf.extend_from_slice(b));
            let mut s: &[u8] = buf.as_slice();
            let decoded = decode_u64(&mut s);
            assert_eq!(decoded, Some(v), "value {v} roundtrip");
            assert!(s.is_empty(), "buffer not fully consumed for {v}");
        }
    }

    #[test]
    fn roundtrip_edge_values() {
        let values = [0_u64, 127, 128, 16383, 16384, u64::MAX];
        for &v in &values {
            let mut buf = DynBuf::new();
            encode_u64(v, &mut |b| buf.extend_from_slice(b));
            let mut s: &[u8] = &buf;
            let decoded = decode_u64(&mut s);
            assert_eq!(decoded, Some(v));
            assert!(s.is_empty());
        }
    }

    #[test]
    fn encoding_shape_examples() {
        let mut buf = DynBuf::new();
        encode_u64(0, &mut |b| buf.extend_from_slice(b));
        assert_eq!(&buf[..], &[0x00]);

        buf.clear();
        encode_u64(127, &mut |b| buf.extend_from_slice(b));
        assert_eq!(&buf[..], &[0x7F]);

        buf.clear();
        encode_u64(128, &mut |b| buf.extend_from_slice(b));
        assert_eq!(&buf[..], &[0x00, 0x81]);

        buf.clear();
        encode_u64(300, &mut |b| buf.extend_from_slice(b));
        assert_eq!(&buf[..], &[0x2C, 0x82]);

        buf.clear();
        encode_u64(16383, &mut |b| buf.extend_from_slice(b));
        assert_eq!(&buf[..], &[0x7F, 0xFF]);

        buf.clear();
        encode_u64(16384, &mut |b| buf.extend_from_slice(b));
        assert_eq!(&buf[..], &[0x00, 0x80, 0x81]);

        buf.clear();
        encode_u64(u64::MAX, &mut |b| buf.extend_from_slice(b));
        assert_eq!(
            &buf[..],
            &[0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x81]
        );
    }

    #[test]
    fn decode_malformed_no_terminator() {
        // Only continuation bytes, never a terminating MSB=0 byte.
        let mut s: &[u8] = &[0x80, 0x80];
        let v = decode_u64(&mut s);
        assert_eq!(v, None);
        assert!(s.is_empty());
    }

    #[test]
    fn encoded_size_matches_actual() {
        let test_values = [
            0_u64,
            1,
            42,
            127,
            128,
            300,
            16383,
            16384,
            1_000_000,
            2_u64.pow(32) - 1,
            2_u64.pow(32),
            u64::MAX,
        ];
        for &v in &test_values {
            let mut buf = DynBuf::new();
            encode_u64(v, &mut |b| buf.extend_from_slice(b));
            let computed_size = encoded_size_u64(v);
            assert_eq!(buf.len() as u64, computed_size, "value {v} size mismatch");
        }
    }
}
