//! Black-box integration tests for the `surp` CLI binary's RFC-001
//! commands: `rfc-compile`, `rfc-inspect`, and `rfc-query`.
//!
//! Like `cli_integration.rs`, these tests drive the real compiled binary
//! through `assert_cmd`/`Command::cargo_bin`, so they remain valid
//! regardless of internal refactors as long as external CLI behavior is
//! unchanged.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

fn example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("data")
        .join(name)
}

fn surp_cmd() -> Command {
    Command::cargo_bin("surp").expect("surp binary should build")
}

/// Minimal CTN fixture that does not use any symbol ('sym) values, so it
/// can round-trip successfully with `--no-symtab`.
const SIMPLE_CTN: &str = "@surp v1\n@encoding cbf\n\nlet simple = Item\n  id = 1\n  name = \"test\"\n\n&simple\n";

#[test]
fn rfc_compile_creates_a_crb_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user.crb");

    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .assert()
        .success()
        .stderr(predicate::str::contains("Wrote"));

    assert!(out_crb.exists());
    let bytes = fs::read(&out_crb).unwrap();
    assert!(!bytes.is_empty());
    // RFC-001 CBF magic.
    assert_eq!(&bytes[0..4], b"SURP");
}

#[test]
fn rfc_compile_rfc_inspect_roundtrip_reports_header_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user.crb");

    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .assert()
        .success();

    surp_cmd()
        .arg("rfc-inspect")
        .arg(&out_crb)
        .assert()
        .success()
        .stdout(predicate::str::contains("Magic: SURP"))
        .stdout(predicate::str::contains("CBF version: 1"))
        .stdout(predicate::str::contains("CTN version: 1"))
        .stdout(predicate::str::contains("has_symtab:      true"))
        .stdout(predicate::str::contains("Symbol count:"));
}

#[test]
fn rfc_compile_rfc_inspect_ctn_roundtrip_decodes_values() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user.crb");

    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .assert()
        .success();

    surp_cmd()
        .arg("rfc-inspect")
        .arg(&out_crb)
        .arg("--ctn")
        .assert()
        .success()
        .stdout(predicate::str::contains("User"))
        .stdout(predicate::str::contains(
            "id = uid\"550e8400-e29b-41d4-a716-446655440000\"",
        ))
        .stdout(predicate::str::contains("name = \"Alice\""))
        .stdout(predicate::str::contains("tags = [\"admin\", \"ops\"]"));
}

#[test]
fn rfc_compile_no_symtab_succeeds_on_symbol_free_document() {
    let dir = tempfile::tempdir().expect("tempdir");
    let in_ctn = dir.path().join("simple.ctn");
    fs::write(&in_ctn, SIMPLE_CTN).unwrap();
    let out_crb = dir.path().join("simple.crb");

    surp_cmd()
        .arg("rfc-compile")
        .arg(&in_ctn)
        .arg("-o")
        .arg(&out_crb)
        .arg("--no-symtab")
        .assert()
        .success();

    surp_cmd()
        .arg("rfc-inspect")
        .arg(&out_crb)
        .assert()
        .success()
        .stdout(predicate::str::contains("has_symtab:      false"))
        .stdout(predicate::str::contains("Symbol count: 0"));
}

#[test]
fn rfc_compile_no_symtab_fails_cleanly_when_document_needs_symbols() {
    // examples/data/user.ctn uses symbol values (e.g. 'Admin, 'region),
    // which require a symbol table. Compiling with --no-symtab must fail
    // with a clear error rather than panicking or silently corrupting
    // output.
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user_nosymtab.crb");

    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .arg("--no-symtab")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("panic").not());

    assert!(
        !out_crb.exists(),
        "no output file should be written when compilation fails"
    );
}

#[test]
fn rfc_query_last_tag_selector() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user.crb");
    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .assert()
        .success();

    surp_cmd()
        .arg("rfc-query")
        .arg(&out_crb)
        .arg(".tags[-1]")
        .assert()
        .success()
        .stdout(predicate::str::diff("\"ops\"\n"));
}

#[test]
fn rfc_query_field_selector() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user.crb");
    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .assert()
        .success();

    surp_cmd()
        .arg("rfc-query")
        .arg(&out_crb)
        .arg(".name")
        .assert()
        .success()
        .stdout(predicate::str::diff("\"Alice\"\n"));
}

#[test]
fn rfc_query_missing_path_returns_null() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user.crb");
    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .assert()
        .success();

    surp_cmd()
        .arg("rfc-query")
        .arg(&out_crb)
        .arg(".nonexistent")
        .assert()
        .success()
        .stdout(predicate::str::diff("null\n"));
}

#[test]
fn rfc_query_via_stdin_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user.crb");
    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .assert()
        .success();

    let bytes = fs::read(&out_crb).unwrap();
    surp_cmd()
        .arg("rfc-query")
        .arg("-")
        .arg(".name")
        .write_stdin(bytes)
        .assert()
        .success()
        .stdout(predicate::str::diff("\"Alice\"\n"));
}

#[test]
fn rfc_compile_stdin_without_explicit_output_is_rejected_cleanly() {
    let bytes = fs::read(example_path("user.ctn")).unwrap();
    surp_cmd()
        .arg("rfc-compile")
        .arg("-")
        .write_stdin(bytes)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("stdin"));
}

#[test]
fn rfc_inspect_on_garbage_input_fails_cleanly_not_panicking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let garbage_path = dir.path().join("garbage.crb");
    fs::write(&garbage_path, b"not a real cbf file at all").unwrap();

    surp_cmd()
        .arg("rfc-inspect")
        .arg(&garbage_path)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("panic").not());
}

#[test]
fn rfc_inspect_on_truncated_crb_fails_cleanly_not_panicking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_crb = dir.path().join("user.crb");
    surp_cmd()
        .arg("rfc-compile")
        .arg(example_path("user.ctn"))
        .arg("-o")
        .arg(&out_crb)
        .assert()
        .success();

    let full = fs::read(&out_crb).unwrap();
    let truncated_path = dir.path().join("truncated.crb");
    fs::write(&truncated_path, &full[..full.len().min(20)]).unwrap();

    surp_cmd()
        .arg("rfc-inspect")
        .arg(&truncated_path)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("panic").not());
}

#[test]
fn rfc_query_on_garbage_input_fails_cleanly_not_panicking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let garbage_path = dir.path().join("garbage.crb");
    fs::write(&garbage_path, b"definitely not cbf").unwrap();

    surp_cmd()
        .arg("rfc-query")
        .arg(&garbage_path)
        .arg(".anything")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("panic").not());
}
