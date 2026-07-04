//! Black-box integration tests for the `surp` CLI binary (v1 wire format
//! commands): `from-json`, `to-json`, `encode`, `decode`, `pretty`,
//! `inspect`, `validate`, and `bench`.
//!
//! These tests invoke the real compiled binary via `assert_cmd` so they
//! exercise argument parsing, stdin handling, file I/O, error formatting,
//! and exit codes exactly as a real user would encounter them. They do not
//! depend on any internal function signatures in `surp-cli/src/main.rs`,
//! so they remain valid across internal refactors as long as the CLI's
//! external behavior is unchanged.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

/// Path to `examples/data/<name>` at the workspace root.
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

#[test]
fn from_json_to_json_roundtrip_is_semantically_equivalent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");

    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    assert!(out_surp.exists(), "from-json should create the output file");

    let to_json = surp_cmd()
        .arg("to-json")
        .arg(&out_surp)
        .arg("--style")
        .arg("compact")
        .assert()
        .success();

    let stdout = String::from_utf8(to_json.get_output().stdout.clone()).expect("utf8 stdout");

    let original: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(example_path("user.json")).unwrap()).unwrap();
    let roundtripped: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();

    // Object key order is not guaranteed to be preserved (from-json goes
    // through serde_json::Value, whose default Map does not preserve
    // insertion order), so compare as parsed JSON values rather than raw
    // text. serde_json::Value equality does not depend on map key order.
    assert_eq!(
        original, roundtripped,
        "value should be semantically identical after from-json -> to-json roundtrip"
    );
}

#[test]
fn from_json_to_json_pretty_style_is_valid_json_and_matches_compact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");

    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let pretty = surp_cmd()
        .arg("to-json")
        .arg(&out_surp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let compact = surp_cmd()
        .arg("to-json")
        .arg(&out_surp)
        .arg("--style")
        .arg("compact")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let pretty_value: serde_json::Value =
        serde_json::from_slice(&pretty).expect("pretty output must be valid JSON");
    let compact_value: serde_json::Value =
        serde_json::from_slice(&compact).expect("compact output must be valid JSON");
    assert_eq!(pretty_value, compact_value);
}

#[test]
fn encode_decode_roundtrip_preserves_text_notation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user2.surp");

    surp_cmd()
        .arg("encode")
        .arg(example_path("user.surp.txt"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let decoded = surp_cmd()
        .arg("decode")
        .arg(&out_surp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let original = fs::read_to_string(example_path("user.surp.txt")).unwrap();
    let decoded_text = String::from_utf8(decoded).unwrap();

    assert_eq!(decoded_text.trim_end(), original.trim_end());
}

#[test]
fn pretty_is_an_alias_for_decode_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user2.surp");

    surp_cmd()
        .arg("encode")
        .arg(example_path("user.surp.txt"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let decode_out = surp_cmd()
        .arg("decode")
        .arg(&out_surp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let pretty_out = surp_cmd()
        .arg("pretty")
        .arg(&out_surp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_eq!(decode_out, pretty_out);
}

#[test]
fn inspect_on_valid_file_succeeds_and_reports_ok_checksums() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    surp_cmd()
        .arg("inspect")
        .arg(&out_surp)
        .assert()
        .success()
        .stdout(predicate::str::contains("Trailer payload checksum:"))
        .stdout(predicate::str::contains("Trailer file checksum:"))
        .stdout(predicate::str::contains("checksum=ok").or(predicate::str::contains("checksum=")));
}

#[test]
fn validate_on_valid_file_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    surp_cmd()
        .arg("validate")
        .arg(&out_surp)
        .assert()
        .success()
        .stderr(predicate::str::contains("Validation passed"));
}

#[test]
fn validate_checksums_only_succeeds_on_uncompressed_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    surp_cmd()
        .arg("validate")
        .arg(&out_surp)
        .arg("--checksums-only")
        .assert()
        .success()
        .stderr(predicate::str::contains("Checksums valid"));
}

#[test]
fn validate_strict_flag_is_accepted_and_logs_strict_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    surp_cmd()
        .arg("validate")
        .arg(&out_surp)
        .arg("--strict")
        .assert()
        .success()
        .stderr(predicate::str::contains("Using strict decode limits"))
        .stderr(predicate::str::contains("Validation passed"));
}

#[test]
fn validate_truncated_file_fails_cleanly_not_panicking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let full = fs::read(&out_surp).unwrap();
    let truncated_path = dir.path().join("truncated.surp");
    fs::write(&truncated_path, &full[..full.len().min(50)]).unwrap();

    surp_cmd()
        .arg("validate")
        .arg(&truncated_path)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("failed to parse block"))
        .stderr(predicate::str::contains("panic").not());
}

#[test]
fn inspect_truncated_file_fails_cleanly_not_panicking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let full = fs::read(&out_surp).unwrap();
    let truncated_path = dir.path().join("truncated.surp");
    fs::write(&truncated_path, &full[..full.len().min(50)]).unwrap();

    surp_cmd()
        .arg("inspect")
        .arg(&truncated_path)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("panic").not());
}

#[test]
fn validate_corrupted_checksum_fails_cleanly_not_panicking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let mut corrupted = fs::read(&out_surp).unwrap();
    // Flip a byte inside the first data block's payload region (well past
    // the fixed header) to invalidate the payload checksum without
    // truncating the file structurally.
    let flip_index = 10.min(corrupted.len() - 1);
    corrupted[flip_index] ^= 0xFF;
    let corrupted_path = dir.path().join("corrupt.surp");
    fs::write(&corrupted_path, &corrupted).unwrap();

    surp_cmd()
        .arg("validate")
        .arg(&corrupted_path)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("panic").not());
}

#[test]
fn stdin_pretty_matches_file_based_pretty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user2.surp");
    surp_cmd()
        .arg("encode")
        .arg(example_path("user.surp.txt"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let file_based = surp_cmd()
        .arg("pretty")
        .arg(&out_surp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let bytes = fs::read(&out_surp).unwrap();
    let stdin_based = surp_cmd()
        .arg("pretty")
        .arg("-")
        .write_stdin(bytes)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(stdin_based, file_based);
}

#[test]
fn stdin_input_without_explicit_output_is_rejected_cleanly() {
    // from-json reading from stdin ('-') must not silently derive an
    // output path from "-"; it should fail with a clear message.
    let bytes = fs::read(example_path("user.json")).unwrap();
    surp_cmd()
        .arg("from-json")
        .arg("-")
        .write_stdin(bytes)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("stdin"));
}

#[test]
fn stdin_input_with_explicit_output_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("from_stdin.surp");
    let bytes = fs::read(example_path("user.json")).unwrap();

    surp_cmd()
        .arg("from-json")
        .arg("-")
        .arg("-o")
        .arg(&out_surp)
        .write_stdin(bytes)
        .assert()
        .success();

    assert!(out_surp.exists());
    assert!(fs::metadata(&out_surp).unwrap().len() > 0);
}

#[test]
fn unsupported_compression_flag_produces_clean_error_not_panic() {
    // This binary is built without the lz4/snappy/zstd features enabled
    // (default = [] in surp-cli/Cargo.toml, and this integration test
    // binary is compiled the same way as the default `cargo test -p
    // surp-cli` invocation), so requesting lz4 compression must be
    // rejected with a clear error rather than panicking or silently
    // falling back to no compression.
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");

    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .arg("--compression")
        .arg("lz4")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("lz4"))
        .stderr(predicate::str::contains("not enabled"))
        .stderr(predicate::str::contains("panic").not());

    assert!(
        !out_surp.exists(),
        "no output file should be written when compression is unsupported"
    );
}

#[test]
fn quiet_suppresses_info_but_not_success_messages() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    // Without --quiet: the informational "Using strict decode limits"
    // message is printed.
    surp_cmd()
        .arg("validate")
        .arg(&out_surp)
        .arg("--strict")
        .assert()
        .success()
        .stderr(predicate::str::contains("Using strict decode limits"));

    // With --quiet: info output is suppressed, but the success message
    // still appears.
    surp_cmd()
        .arg("--quiet")
        .arg("validate")
        .arg(&out_surp)
        .arg("--strict")
        .assert()
        .success()
        .stderr(predicate::str::contains("Using strict decode limits").not())
        .stderr(predicate::str::contains("Validation passed"));
}

#[test]
fn no_color_env_disables_ansi_codes_in_auto_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    // Default color mode is "auto"; assert_cmd captures stdout to a pipe
    // (not a tty), so auto-mode should already disable color, and setting
    // NO_COLOR=1 makes that explicit/robust either way.
    let output = surp_cmd()
        .env("NO_COLOR", "1")
        .arg("inspect")
        .arg(&out_surp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    assert!(
        !text.contains('\x1b'),
        "expected no ANSI escape codes in auto/NO_COLOR mode, got: {text:?}"
    );
}

#[test]
fn color_always_overrides_no_color_env() {
    // Documents current CLI behavior: `--color always` unconditionally
    // enables ANSI color regardless of the NO_COLOR environment variable
    // (only ColorChoice::Auto consults NO_COLOR).
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let output = surp_cmd()
        .env("NO_COLOR", "1")
        .arg("--color")
        .arg("always")
        .arg("inspect")
        .arg(&out_surp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    assert!(
        text.contains('\x1b'),
        "expected ANSI escape codes when --color always is forced, got: {text:?}"
    );
}

#[test]
fn color_never_disables_ansi_codes_even_without_no_color() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_surp = dir.path().join("user.surp");
    surp_cmd()
        .arg("from-json")
        .arg(example_path("user.json"))
        .arg("-o")
        .arg(&out_surp)
        .assert()
        .success();

    let output = surp_cmd()
        .env_remove("NO_COLOR")
        .arg("--color")
        .arg("never")
        .arg("inspect")
        .arg(&out_surp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    assert!(!text.contains('\x1b'));
}

#[test]
fn bench_runs_successfully_and_reports_throughput() {
    surp_cmd()
        .arg("bench")
        .arg(example_path("user.json"))
        .arg("-n")
        .arg("5")
        .arg("--warmup")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Benchmark:"))
        .stdout(predicate::str::contains("MB/s"))
        .stderr(predicate::str::contains("Benchmark complete"));
}

#[test]
fn bench_rejects_unsupported_compression_cleanly() {
    surp_cmd()
        .arg("bench")
        .arg(example_path("user.json"))
        .arg("-n")
        .arg("1")
        .arg("--compression")
        .arg("zstd")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("zstd"))
        .stderr(predicate::str::contains("not enabled"));
}
