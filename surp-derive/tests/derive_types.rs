//! Integration tests for `#[derive(Surp)]` with all supported field types.

use surp_core::{Surp, SurpBytes, Value};
use surp_derive::{Surp, SurpSchema};

// ─── Structs using all newly-supported types ─────────────────────────────────

#[derive(Debug, PartialEq, Surp, SurpSchema)]
struct Person {
    #[surp(id = 1)]
    name: String,
    #[surp(id = 2)]
    age: u8,
    #[surp(id = 3)]
    tags: Vec<String>,
}

#[derive(Debug, PartialEq, Surp)]
struct AllIntegers {
    #[surp(id = 1)]
    a: u8,
    #[surp(id = 2)]
    b: u16,
    #[surp(id = 3)]
    c: u32,
    #[surp(id = 4)]
    d: u64,
    #[surp(id = 5)]
    e: i8,
    #[surp(id = 6)]
    f: i16,
    #[surp(id = 7)]
    g: i32,
    #[surp(id = 8)]
    h: i64,
}

#[derive(Debug, PartialEq, Surp)]
struct WithFloats {
    #[surp(id = 1)]
    single: f32,
    #[surp(id = 2)]
    double: f64,
}

#[derive(Debug, PartialEq, Surp)]
struct WithBytes {
    #[surp(id = 1)]
    label: String,
    #[surp(id = 2)]
    raw: SurpBytes,
    #[surp(id = 3)]
    byte_array: Vec<u8>,
}

#[derive(Debug, PartialEq, Surp)]
struct WithOptionals {
    #[surp(id = 1)]
    required: u32,
    #[surp(id = 2)]
    maybe_name: Option<String>,
    #[surp(id = 3)]
    maybe_count: Option<u16>,
}

#[derive(Debug, PartialEq, Surp)]
struct WithBool {
    #[surp(id = 1)]
    flag: bool,
    #[surp(id = 2)]
    items: Vec<bool>,
}

#[derive(Debug, PartialEq, Surp)]
struct WithVecU8 {
    #[surp(id = 1)]
    data: Vec<u8>,
    #[surp(id = 2)]
    name: String,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn person_with_u8_age() {
    let p = Person {
        name: "Alice".into(),
        age: 30,
        tags: vec!["admin".into(), "dev".into()],
    };

    let val = p.to_surp_value();
    let back = Person::from_surp_value(&val).unwrap();
    assert_eq!(p, back);
}

#[test]
fn person_binary_roundtrip() {
    let p = Person {
        name: "Bob".into(),
        age: 255, // max u8
        tags: vec![],
    };

    let bytes = p.to_surp_bytes().unwrap();
    let back = Person::from_surp_bytes(&bytes).unwrap();
    assert_eq!(p, back);
}

#[test]
fn person_schema_info() {
    let info = Person::schema_info();
    assert_eq!(info.len(), 3);
    assert_eq!(info[0], ("name", 1));
    assert_eq!(info[1], ("age", 2));
    assert_eq!(info[2], ("tags", 3));
}

#[test]
fn all_integers_roundtrip() {
    let v = AllIntegers {
        a: 255,
        b: 65535,
        c: u32::MAX,
        d: u64::MAX,
        e: -128,
        f: -32768,
        g: i32::MIN,
        h: i64::MIN,
    };

    let val = v.to_surp_value();
    let back = AllIntegers::from_surp_value(&val).unwrap();
    assert_eq!(v, back);
}

#[test]
fn all_integers_binary_roundtrip() {
    let v = AllIntegers {
        a: 0,
        b: 1000,
        c: 100_000,
        d: 1_000_000,
        e: -1,
        f: 100,
        g: -100_000,
        h: 0,
    };

    let bytes = v.to_surp_bytes().unwrap();
    let back = AllIntegers::from_surp_bytes(&bytes).unwrap();
    assert_eq!(v, back);
}

#[test]
fn with_floats_roundtrip() {
    let v = WithFloats {
        single: 1.5,
        double: 99.99,
    };

    let val = v.to_surp_value();
    let back = WithFloats::from_surp_value(&val).unwrap();
    // f32 loses precision through f64
    assert!((back.single - 1.5).abs() < 1e-6);
    assert!((back.double - 99.99).abs() < 1e-10);
}

#[test]
fn with_surp_bytes_roundtrip() {
    let v = WithBytes {
        label: "payload".into(),
        raw: SurpBytes(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        byte_array: vec![1, 2, 3, 4, 5],
    };

    let val = v.to_surp_value();

    // SurpBytes → Bytes, Vec<u8> → Array.
    // Wire object keys are the stringified numeric `#[surp(id = N)]`, not
    // the field name (raw = id 2, byte_array = id 3 in `WithBytes`).
    if let Value::Object(entries) = &val {
        let raw_val = entries.iter().find(|(k, _)| k == "2").unwrap();
        assert!(matches!(raw_val.1, Value::Bytes(_)));

        let arr_val = entries.iter().find(|(k, _)| k == "3").unwrap();
        assert!(matches!(arr_val.1, Value::Array(_)));
    } else {
        panic!("expected Object");
    }

    let back = WithBytes::from_surp_value(&val).unwrap();
    assert_eq!(v, back);
}

#[test]
fn with_bytes_binary_roundtrip() {
    let v = WithBytes {
        label: "test".into(),
        raw: SurpBytes(vec![0xFF; 100]),
        byte_array: vec![10, 20, 30],
    };

    let bytes = v.to_surp_bytes().unwrap();
    let back = WithBytes::from_surp_bytes(&bytes).unwrap();
    assert_eq!(v, back);
}

#[test]
fn with_optionals_present() {
    let v = WithOptionals {
        required: 42,
        maybe_name: Some("hello".into()),
        maybe_count: Some(100),
    };

    let bytes = v.to_surp_bytes().unwrap();
    let back = WithOptionals::from_surp_bytes(&bytes).unwrap();
    assert_eq!(v, back);
}

#[test]
fn with_optionals_absent() {
    let v = WithOptionals {
        required: 0,
        maybe_name: None,
        maybe_count: None,
    };

    let bytes = v.to_surp_bytes().unwrap();
    let back = WithOptionals::from_surp_bytes(&bytes).unwrap();
    assert_eq!(v, back);
}

#[test]
fn with_bool_roundtrip() {
    let v = WithBool {
        flag: true,
        items: vec![true, false, true, false],
    };

    let bytes = v.to_surp_bytes().unwrap();
    let back = WithBool::from_surp_bytes(&bytes).unwrap();
    assert_eq!(v, back);
}

#[test]
fn vec_u8_field_binary_roundtrip() {
    let v = WithVecU8 {
        data: vec![0, 127, 255, 1, 42],
        name: "binary-array".into(),
    };

    let bytes = v.to_surp_bytes().unwrap();
    let back = WithVecU8::from_surp_bytes(&bytes).unwrap();
    assert_eq!(v, back);
}

#[test]
fn fingerprint_is_stable() {
    // Schema fingerprint should be deterministic.
    let fp1 = Person::schema_fingerprint();
    let fp2 = Person::schema_fingerprint();
    assert_eq!(fp1, fp2);
    assert_ne!(fp1, 0);

    // Different structs should have different fingerprints.
    let fp3 = AllIntegers::schema_fingerprint();
    assert_ne!(fp1, fp3);
}

// ─── Regression: field ids drive wire encoding, not field names ─────────────
//
// `RenameOld`/`RenameNew` model the same logical schema (same ids, same
// types, in the same order) before and after a field rename. Because
// `to_surp_value`/`from_surp_value` now key the `Value::Object` entries by
// the stringified `#[surp(id = N)]` instead of the Rust field name, renaming
// a field while keeping its id must produce byte-identical wire output. This
// is the "stable field IDs" guarantee the crate's docs advertise.

#[derive(Debug, PartialEq, Surp)]
struct RenameOld {
    #[surp(id = 1)]
    first_name: String,
    #[surp(id = 2)]
    age: u8,
}

#[derive(Debug, PartialEq, Surp)]
struct RenameNew {
    #[surp(id = 1)]
    full_name: String, // renamed from `first_name`, same id
    #[surp(id = 2)]
    age: u8,
}

#[test]
fn renaming_field_keeps_same_id_produces_identical_wire_bytes() {
    let old = RenameOld {
        first_name: "Alice".into(),
        age: 30,
    };
    let new = RenameNew {
        full_name: "Alice".into(),
        age: 30,
    };

    // Same Value representation: object keys are ids ("1", "2"), not names.
    assert_eq!(old.to_surp_value(), new.to_surp_value());

    // Same encoded bytes end-to-end.
    let old_bytes = old.to_surp_bytes().unwrap();
    let new_bytes = new.to_surp_bytes().unwrap();
    assert_eq!(old_bytes, new_bytes);

    // And the renamed struct can decode bytes produced by the old struct.
    let decoded = RenameNew::from_surp_bytes(&old_bytes).unwrap();
    assert_eq!(decoded, new);
}

// ─── Regression: fallback id agreement between Surp and SurpSchema ──────────
//
// When a field omits `#[surp(id = N)]`, `derive_surp` and `derive_surp_schema`
// must resolve the *same* fallback id (an xxh64 hash of the field name),
// since both are folded into `schema_fingerprint()` and `schema_info()`
// respectively. Before the shared `resolve_field_id` helper,
// `derive_surp_schema` always reported a fallback id of `0`, which
// disagreed with the hash-derived id `derive_surp` actually used on the
// wire and in the fingerprint.

#[derive(Debug, PartialEq, Surp, SurpSchema)]
struct WithImplicitId {
    #[surp(id = 1)]
    explicit: u32,
    // No `#[surp(id = ..)]`: both macros must fall back to the same
    // xxh64-derived id for `implicit_field`.
    implicit_field: u32,
}

// A second struct whose only difference from `WithImplicitId` is that the
// implicit field has an explicit id equal to 0 (the value the buggy
// `derive_surp_schema` fallback always reported). If the fallback ids ever
// disagree again, this struct's fingerprint would accidentally match
// `WithImplicitId`'s even though the wire ids differ.
#[derive(Debug, PartialEq, Surp, SurpSchema)]
struct WithExplicitZeroId {
    #[surp(id = 1)]
    explicit: u32,
    #[surp(id = 0)]
    implicit_field: u32,
}

#[test]
fn schema_info_id_matches_fingerprint_input_id_for_implicit_fields() {
    let info = WithImplicitId::schema_info();
    let reported_id = info
        .iter()
        .find(|(name, _)| *name == "implicit_field")
        .map(|(_, id)| *id)
        .expect("implicit_field present in schema_info");

    // The id `derive_surp` actually folds into the fingerprint for an
    // implicit field is the same xxh64-of-name fallback used here.
    let expected_id =
        xxhash_rust::xxh64::xxh64("implicit_field".as_bytes(), 0) & 0xFFFF;
    assert_eq!(reported_id, expected_id);

    // Cross-check against the actual bug scenario: if `schema_info()` still
    // (incorrectly) reported 0 for the implicit field, it would equal the
    // explicit-id-0 struct's reported id even though the two structs use
    // different wire ids and therefore must have different fingerprints.
    assert_ne!(reported_id, 0);
    assert_ne!(
        WithImplicitId::schema_fingerprint(),
        WithExplicitZeroId::schema_fingerprint()
    );

    // And the wire bytes for the implicit-id struct must actually use the
    // resolved hash id as the object key (not "0" and not the field name).
    let v = WithImplicitId {
        explicit: 1,
        implicit_field: 2,
    };
    if let Value::Object(entries) = v.to_surp_value() {
        let key = format!("{expected_id}");
        assert!(entries.iter().any(|(k, _)| *k == key));
        assert!(!entries.iter().any(|(k, _)| k == "implicit_field"));
    } else {
        panic!("expected Object");
    }
}
