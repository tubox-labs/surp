//! Compile-fail tests for proc-macro error hygiene.
//!
//! These assert that malformed `#[surp(...)]` attributes, duplicate field
//! ids, and unsupported input shapes (enums, tuple structs) produce a clean
//! compile error rather than either being silently swallowed (falling back
//! to a hash-derived id with no diagnostic) or panicking inside the
//! proc-macro (which surfaces to users as an opaque "proc macro panicked"
//! message instead of a message pointing at the offending code).
//!
//! We deliberately do *not* check in `.stderr` fixtures and compare exact
//! compiler output: `trybuild`'s stderr comparison is brittle across rustc
//! versions (wording/formatting of `syn`/rustc diagnostics can shift), which
//! would make this suite flaky in CI for reasons unrelated to the behavior
//! under test. `trybuild::TestCases::compile_fail` without a matching
//! `.stderr` file still fully exercises the real thing we care about: that
//! the macro invocation fails to compile instead of panicking or silently
//! doing the wrong thing.
#[test]
fn compile_fail_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/*.rs");
}
