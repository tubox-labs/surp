//! # surp-ffi
//!
//! C-compatible FFI bindings for the Surp encoder/decoder.
//!
//! Memory ownership:
//! - `surp_encode_buffer`: caller provides input, library allocates output.
//!   Caller must free output with `surp_free`.
//! - `surp_decode_buffer`: caller provides Surp binary input, library allocates
//!   JSON string output. Caller must free with `surp_free`.
//! - `surp_free`: frees memory allocated by this library.

use std::slice;

/// Encode a JSON string to Surp binary format.
///
/// # Safety
/// - `in_ptr` must point to `in_len` valid bytes of JSON text.
/// - `out_ptr` and `out_len` must be valid, non-null pointers.
/// - Caller must free `*out_ptr` with `surp_free`.
///
/// Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn surp_encode_buffer(
    in_ptr: *const u8,
    in_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if in_ptr.is_null() || out_ptr.is_null() || out_len.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let input = unsafe { slice::from_raw_parts(in_ptr, in_len) };

        // Parse JSON input.
        let json_str = match std::str::from_utf8(input) {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let json_value: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return -1,
        };

        let surp_value = surp_core::Value::from(&json_value);

        let mut encoder = surp_core::Encoder::new();
        if encoder.encode_value(&surp_value).is_err() {
            return -1;
        }

        let bytes = match encoder.finish() {
            Ok(b) => b,
            Err(_) => return -1,
        };

        // Allocate output buffer.
        let boxed = bytes.into_boxed_slice();
        let len = boxed.len();
        let raw = Box::into_raw(boxed) as *mut u8;

        unsafe {
            *out_ptr = raw;
            *out_len = len;
        }

        0
    }));

    result.unwrap_or(-1)
}

/// Decode a Surp binary buffer to a JSON string.
///
/// # Safety
/// - `in_ptr` must point to `in_len` valid bytes of Surp binary data.
/// - `json_out` and `json_len` must be valid, non-null pointers.
/// - Caller must free `*json_out` with `surp_free`.
///
/// Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn surp_decode_buffer(
    in_ptr: *const u8,
    in_len: usize,
    json_out: *mut *mut u8,
    json_len: *mut usize,
) -> i32 {
    if in_ptr.is_null() || json_out.is_null() || json_len.is_null() {
        return -1;
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let input = unsafe { slice::from_raw_parts(in_ptr, in_len) };

        let mut decoder = surp_core::Decoder::new(input);
        let values = match decoder.decode_all_owned() {
            Ok(v) => v,
            Err(_) => return -1,
        };

        // Convert to JSON.
        let json_values: Vec<serde_json::Value> =
            values.iter().map(serde_json::Value::from).collect();
        let json_string = if json_values.len() == 1 {
            serde_json::to_string_pretty(&json_values[0]).unwrap_or_default()
        } else {
            serde_json::to_string_pretty(&json_values).unwrap_or_default()
        };

        let bytes = json_string.into_bytes().into_boxed_slice();
        let len = bytes.len();
        let raw = Box::into_raw(bytes) as *mut u8;

        unsafe {
            *json_out = raw;
            *json_len = len;
        }

        0
    }));

    result.unwrap_or(-1)
}

/// Free memory allocated by `surp_encode_buffer` or `surp_decode_buffer`.
///
/// # Safety
/// - `ptr` must have been allocated by this library, or be null (no-op).
///
/// Note: unlike `surp_encode_buffer`/`surp_decode_buffer`, this function does
/// not call into `surp_core` or `serde_json` and only drops a boxed `[u8]`
/// (whose destructor cannot panic), so it is not wrapped in `catch_unwind`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn surp_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn ffi_encode_decode_roundtrip() {
        let json_input = br#"{"name":"Alice","age":30}"#;

        let mut out_ptr: *mut u8 = ptr::null_mut();
        let mut out_len: usize = 0;

        let rc = unsafe {
            surp_encode_buffer(
                json_input.as_ptr(),
                json_input.len(),
                &mut out_ptr,
                &mut out_len,
            )
        };
        assert_eq!(rc, 0);
        assert!(!out_ptr.is_null());
        assert!(out_len > 0);

        // Decode back.
        let mut json_out: *mut u8 = ptr::null_mut();
        let mut json_len: usize = 0;

        let rc2 = unsafe { surp_decode_buffer(out_ptr, out_len, &mut json_out, &mut json_len) };
        assert_eq!(rc2, 0);

        let json_str =
            unsafe { std::str::from_utf8(slice::from_raw_parts(json_out, json_len)).unwrap() };
        assert!(json_str.contains("Alice"));
        assert!(json_str.contains("30"));

        // Free.
        unsafe {
            surp_free(out_ptr, out_len);
            surp_free(json_out, json_len);
        }
    }

    /// Build a well-formed Surp block (correct length + XXH64 checksum, so it
    /// passes `BlockReader`/`read_next_block` validation) whose payload is a
    /// `StartObject` with `count = 1` followed by a key-length varint of
    /// `u64::MAX`. `surp_core::decoder`'s `StartObject` arm computes
    /// `self.block_pos + key_len` without a checked add, which panics with
    /// "attempt to add with overflow" under `overflow-checks` (on by default
    /// in dev/test profiles), and would panic on the resulting bogus slice
    /// range in release profiles. This is a real, reachable panic path for
    /// `surp_decode_buffer`, driven entirely by attacker-controlled input.
    fn malicious_decode_input_with_huge_key_len() -> Vec<u8> {
        use surp_core::block::BlockWriter;
        use surp_core::varint::encode_varint_vec;
        use surp_core::wire::{BlockType, WireType};

        let mut payload = Vec::new();
        payload.push(WireType::StartObject.to_tag());
        encode_varint_vec(1, &mut payload); // object entry count = 1
        encode_varint_vec(u64::MAX, &mut payload); // key length = u64::MAX

        let mut writer = BlockWriter::new(BlockType::Data);
        writer.write(&payload);
        writer.finish()
    }

    #[test]
    fn malicious_input_panics_in_surp_core_directly() {
        // Sanity check: confirm the crafted input really does panic when fed
        // straight into surp_core (i.e. this isn't a no-op/already-handled
        // error case). This proves catch_unwind in surp_decode_buffer is
        // actually doing work, not just wrapping code that never panics.
        let bytes = malicious_decode_input_with_huge_key_len();

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silence expected panic output
        let result = std::panic::catch_unwind(|| {
            let mut decoder = surp_core::Decoder::new(&bytes);
            decoder.decode_all_owned()
        });
        std::panic::set_hook(prev_hook);

        assert!(
            result.is_err(),
            "expected surp_core::Decoder::decode_all_owned to panic on a huge \
             object-key length, but it returned normally"
        );
    }

    #[test]
    fn surp_decode_buffer_survives_panic_and_returns_error_code() {
        // Regression test for the panic guard: without `catch_unwind` inside
        // `surp_decode_buffer`, the panic triggered by this input would
        // unwind straight out of an `extern "C" fn`, which aborts the whole
        // process under current Rust semantics instead of returning -1. With
        // the guard in place, the call must return cleanly with the
        // documented error code.
        let bytes = malicious_decode_input_with_huge_key_len();

        let mut json_out: *mut u8 = ptr::null_mut();
        let mut json_len: usize = 0;

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silence expected panic output
        let rc = unsafe {
            surp_decode_buffer(bytes.as_ptr(), bytes.len(), &mut json_out, &mut json_len)
        };
        std::panic::set_hook(prev_hook);

        assert_eq!(rc, -1, "expected documented error code, not a propagated panic");
        assert!(json_out.is_null());
        assert_eq!(json_len, 0);
    }

    #[test]
    fn ffi_encode_decode_reject_malformed_inputs_without_aborting() {
        // Battery of malformed/edge-case inputs for both entry points. None
        // of these should ever cause the process to abort; every call must
        // return the documented negative error code.
        let bad_encode_inputs: &[&[u8]] = &[
            b"",
            b"not json",
            b"{",
            b"{\"a\":}",
            b"[1,2,",
            &[0xFF, 0xFE, 0xFD], // invalid UTF-8
            b"nul", // truncated literal
        ];

        for input in bad_encode_inputs {
            let mut out_ptr: *mut u8 = ptr::null_mut();
            let mut out_len: usize = 0;
            let rc = unsafe {
                surp_encode_buffer(input.as_ptr(), input.len(), &mut out_ptr, &mut out_len)
            };
            assert_eq!(rc, -1, "expected -1 for malformed encode input {input:?}");
            assert!(out_ptr.is_null());
        }

        // Note: decode_all_owned() deliberately treats a truncated trailing
        // block as UnexpectedEof and stops (rather than erroring), so
        // partial-header byte sequences like `[0x01]` are NOT malformed from
        // this API's perspective — they decode to an empty value list. Use
        // an unmapped block-type byte (see BlockType::from_byte) to trigger
        // a genuine, non-EOF decode error.
        let bad_decode_inputs: &[&[u8]] = &[
            &[0x05], // no BlockType variant maps to 0x05
            &[0xAA, 0xBB, 0xCC, 0xDD],
            &malicious_decode_input_with_huge_key_len(),
        ];

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silence expected panic output
        for input in bad_decode_inputs {
            let mut json_out: *mut u8 = ptr::null_mut();
            let mut json_len: usize = 0;
            let rc = unsafe {
                surp_decode_buffer(input.as_ptr(), input.len(), &mut json_out, &mut json_len)
            };
            assert_eq!(rc, -1, "expected -1 for malformed decode input {input:?}");
            assert!(json_out.is_null());
        }
        std::panic::set_hook(prev_hook);
    }
}
