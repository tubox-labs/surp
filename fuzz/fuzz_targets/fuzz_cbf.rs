//! Fuzz target: RFC-001 CBF binary decoder.
//!
//! Feeds arbitrary bytes to `surp_core::rfc001::decode_document` to verify
//! the CBF decoder never panics, hangs, or allocates unboundedly on
//! malformed/adversarial binary input (e.g. crafted length fields in the
//! header, symbol table, or value payload).
//!
//! Run with: cargo +nightly fuzz run fuzz_cbf

#![no_main]

use libfuzzer_sys::fuzz_target;
use surp_core::rfc001;

fuzz_target!(|data: &[u8]| {
    // Attempt to decode arbitrary bytes as a CBF document — must not panic.
    let _ = rfc001::decode_document(data);
});
