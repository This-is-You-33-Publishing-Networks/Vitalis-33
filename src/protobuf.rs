//! Protocol Buffers Codec — Systems Infrastructure (v331)
//!
//! Wire format encoding/decoding: varint, length-delimited, fixed-width.
//! Schema-less protobuf primitive operations.

use std::sync::atomic::{AtomicI64, Ordering};
static PB_OPS: AtomicI64 = AtomicI64::new(0);

/// Encode a varint: value → number of bytes needed. Returns byte count (1-10).
#[unsafe(no_mangle)]
pub extern "C" fn slang_protobuf_encode(value: i64) -> i64 {
    PB_OPS.fetch_add(1, Ordering::Relaxed);
    let mut v = value as u64;
    let mut bytes = 0i64;
    loop { bytes += 1; v >>= 7; if v == 0 { break; } }
    bytes
}

/// Decode a varint byte count back to original value.
/// Simplified: returns value if within valid varint range, -1 otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn slang_protobuf_decode(encoded_bytes: i64) -> i64 {
    PB_OPS.fetch_add(1, Ordering::Relaxed);
    if encoded_bytes < 1 || encoded_bytes > 10 { return -1; }
    // Max value representable in N bytes of varint: 2^(7*N) - 1
    if encoded_bytes >= 10 { return i64::MAX; }
    ((1_i64 << (7 * encoded_bytes)) - 1).min(i64::MAX)
}

/// Create a field tag: (field_number << 3) | wire_type.
/// wire_type: 0=varint, 1=fixed64, 2=length-delimited, 5=fixed32.
#[unsafe(no_mangle)]
pub extern "C" fn slang_protobuf_field(field_number: i64, wire_type: i64) -> i64 {
    PB_OPS.fetch_add(1, Ordering::Relaxed);
    (field_number << 3) | (wire_type & 0x7)
}

/// Calculate the serialized size of a message with n_fields, total_data_bytes.
/// Returns total bytes (field tags + data).
#[unsafe(no_mangle)]
pub extern "C" fn slang_protobuf_size(n_fields: i64, total_data_bytes: i64) -> i64 {
    PB_OPS.fetch_add(1, Ordering::Relaxed);
    // Each field: 1 byte tag + data
    n_fields + total_data_bytes.max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_encode_small() { assert_eq!(slang_protobuf_encode(0), 1); }
    #[test] fn test_encode_one_byte() { assert_eq!(slang_protobuf_encode(127), 1); }
    #[test] fn test_encode_two_bytes() { assert_eq!(slang_protobuf_encode(128), 2); }
    #[test] fn test_encode_large() { assert!(slang_protobuf_encode(i64::MAX) <= 10); }
    #[test] fn test_decode_valid() { let v = slang_protobuf_decode(1); assert_eq!(v, 127); }
    #[test] fn test_decode_invalid() { assert_eq!(slang_protobuf_decode(0), -1); }
    #[test] fn test_decode_max() { assert_eq!(slang_protobuf_decode(10), i64::MAX); }
    #[test] fn test_field_varint() { assert_eq!(slang_protobuf_field(1, 0), 8); }
    #[test] fn test_field_length_delim() { assert_eq!(slang_protobuf_field(2, 2), 18); }
    #[test] fn test_size() { assert_eq!(slang_protobuf_size(3, 10), 13); }
    #[test] fn test_size_zero_data() { assert_eq!(slang_protobuf_size(5, 0), 5); }
    #[test] fn test_size_zero_fields() { assert_eq!(slang_protobuf_size(0, 10), 10); }
    #[test] fn test_encode_negative() { let b = slang_protobuf_encode(-1); assert!(b > 0); }
    #[test] fn test_field_wire_type_mask() { assert_eq!(slang_protobuf_field(1, 15) & 0x7, 7); }
    #[test] fn test_field_large_number() { let f = slang_protobuf_field(100, 0); assert_eq!(f >> 3, 100); }
    #[test] fn test_decode_two_bytes() { let v = slang_protobuf_decode(2); assert_eq!(v, 16383); }
    #[test] fn test_encode_boundary() { assert_eq!(slang_protobuf_encode(16383), 2); }
    #[test] fn test_encode_three_bytes() { assert_eq!(slang_protobuf_encode(16384), 3); }
    #[test] fn test_size_negative_data() { assert_eq!(slang_protobuf_size(2, -5), 2); }
    #[test] fn test_field_zero() { assert_eq!(slang_protobuf_field(0, 0), 0); }
    #[test] fn test_decode_eleven() { assert_eq!(slang_protobuf_decode(11), -1); }
}
