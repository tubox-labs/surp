//! Fuzz target: varint encoding/decoding.
//!
//! Ensures varint codec never panics on arbitrary input bytes,
//! and that encode → decode roundtrips correctly for any u64 value.
//!
//! Run with: cargo +nightly fuzz run fuzz_varint

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try decoding arbitrary bytes — must not panic.
    let _ = surp_core::varint::decode_varint(data, 0);

    // If we have at least 8 bytes, interpret them as a u64 and roundtrip.
    if data.len() >= 8 {
        let value = u64::from_le_bytes(data[..8].try_into().unwrap());

        // Unsigned roundtrip
        let mut buf = Vec::new();
        surp_core::varint::encode_varint_vec(value, &mut buf);
        let (decoded, _) =
            surp_core::varint::decode_varint(&buf, 0).expect("valid varint");
        assert_eq!(value, decoded);

        // Signed roundtrip (zigzag)
        let signed = value as i64;
        let encoded = surp_core::varint::zigzag_encode(signed);
        let decoded = surp_core::varint::zigzag_decode(encoded);
        assert_eq!(signed, decoded);
    }
});
