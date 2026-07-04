//! Fuzz target: RFC-001 CQL baseline query engine.
//!
//! Generates a fuzzer-controlled CTN source document alongside a
//! fuzzer-controlled query expression, parses the document, resolves its
//! effective root, and runs the query against it. Verifies the CQL
//! selector parser and structural traversal never panic regardless of
//! how malformed the query expression or source document are.
//!
//! Run with: cargo +nightly fuzz run fuzz_cql

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use surp_core::rfc001;

#[derive(Debug, Arbitrary)]
struct CqlInput {
    /// Fuzzer-controlled CTN source to parse into a document.
    ctn_source: String,
    /// Fuzzer-controlled CQL path expression to evaluate against the root.
    query: String,
}

fuzz_target!(|input: CqlInput| {
    let doc = match rfc001::parse_document(&input.ctn_source) {
        Ok(doc) => doc,
        Err(_) => return,
    };
    let root = match doc.effective_root() {
        Ok(root) => root,
        Err(_) => return,
    };

    // Must not panic regardless of query string content.
    let _ = rfc001::query(&root, &input.query);
    let _ = rfc001::query_one(&root, &input.query);
});
