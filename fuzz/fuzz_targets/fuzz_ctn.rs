//! Fuzz target: RFC-001 CTN text parser.
//!
//! Feeds arbitrary strings to the RFC-001 CTN document parser to verify
//! it never panics or allocates unboundedly on malformed/adversarial
//! input, and that any successfully parsed document can be formatted
//! back out and re-parsed without panicking.
//!
//! Run with: cargo +nightly fuzz run fuzz_ctn

#![no_main]

use libfuzzer_sys::fuzz_target;
use surp_core::rfc001;

fuzz_target!(|data: &[u8]| {
    // Only feed valid UTF-8 to the CTN parser — matches CTN's text nature.
    if let Ok(text) = std::str::from_utf8(data) {
        // Must not panic on any input.
        if let Ok(doc) = rfc001::parse_document(text) {
            // If parsing succeeds, formatting must also succeed without panicking.
            let printed = rfc001::format_document(&doc);
            // Re-parsing the formatted output should not panic either.
            let _ = rfc001::parse_document(&printed);
        }
    }
});
