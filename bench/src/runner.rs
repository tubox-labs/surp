//! Benchmark runner — executes measurements for each format × dataset × operation.

use std::time::Instant;

use prost::Message;
use surp_core::{Decoder, Encoder, Value};

use crate::datasets::Dataset;
use crate::metrics::Measurement;

/// Number of warm-up iterations before measuring.
const WARMUP: usize = 3;

/// Run all measurements for a single dataset across all formats.
pub fn run_dataset(dataset: &Dataset, iterations: usize) -> Vec<Measurement> {
    let mut results = Vec::new();

    // ── Surp ────────────────────────────────────────────────────────
    let surp_bytes = surp_encode_once(&dataset.value);
    results.push(bench_surp_encode(dataset, iterations, &surp_bytes));
    results.push(bench_surp_decode(dataset, iterations, &surp_bytes));
    results.push(bench_surp_roundtrip(dataset, iterations));

    // ── Surp + string dedup ─────────────────────────────────────────
    let dedup_bytes = surp_encode_dedup_once(&dataset.value);
    results.push(bench_surp_dedup_encode(dataset, iterations, &dedup_bytes));
    results.push(bench_surp_dedup_decode(dataset, iterations, &dedup_bytes));

    // ── JSON (serde_json) ────────────────────────────────────────────
    let json_val: serde_json::Value = (&dataset.value).into();
    let json_bytes = serde_json::to_vec(&json_val).unwrap();
    results.push(bench_json_encode(
        dataset,
        iterations,
        &json_val,
        &json_bytes,
    ));
    results.push(bench_json_decode(dataset, iterations, &json_bytes));
    results.push(bench_json_roundtrip(dataset, iterations, &json_val));

    // ── MessagePack (rmp-serde) ──────────────────────────────────────
    let msgpack_bytes = rmp_serde::to_vec(&json_val).unwrap();
    results.push(bench_msgpack_encode(
        dataset,
        iterations,
        &json_val,
        &msgpack_bytes,
    ));
    results.push(bench_msgpack_decode(dataset, iterations, &msgpack_bytes));
    results.push(bench_msgpack_roundtrip(dataset, iterations, &json_val));

    // ── CBOR (ciborium) ──────────────────────────────────────────────
    let mut cbor_bytes = Vec::new();
    ciborium::into_writer(&json_val, &mut cbor_bytes).unwrap();
    results.push(bench_cbor_encode(
        dataset,
        iterations,
        &json_val,
        &cbor_bytes,
    ));
    results.push(bench_cbor_decode(dataset, iterations, &cbor_bytes));
    results.push(bench_cbor_roundtrip(dataset, iterations, &json_val));

    // -- Protocol Buffers (generic Value schema) ---------------------
    let proto_value = ProtoValue::from_surp_value(&dataset.value);
    let protobuf_bytes = protobuf_encode_once(&proto_value);
    results.push(bench_protobuf_encode(
        dataset,
        iterations,
        &proto_value,
        &protobuf_bytes,
    ));
    results.push(bench_protobuf_decode(dataset, iterations, &protobuf_bytes));
    results.push(bench_protobuf_roundtrip(dataset, iterations, &proto_value));

    results
}

// -- Protocol Buffers model -----------------------------------------

#[derive(Clone, PartialEq, Message)]
struct ProtoValue {
    #[prost(oneof = "proto_value::Kind", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
    kind: Option<proto_value::Kind>,
}

mod proto_value {
    use prost::Oneof;

    use super::{ProtoArray, ProtoObject};

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Kind {
        #[prost(bool, tag = "1")]
        Null(bool),
        #[prost(bool, tag = "2")]
        Bool(bool),
        #[prost(uint64, tag = "3")]
        UInt(u64),
        #[prost(sint64, tag = "4")]
        Int(i64),
        #[prost(double, tag = "5")]
        Float(f64),
        #[prost(string, tag = "6")]
        Str(String),
        #[prost(bytes, tag = "7")]
        Bytes(Vec<u8>),
        #[prost(message, tag = "8")]
        Array(ProtoArray),
        #[prost(message, tag = "9")]
        Object(ProtoObject),
    }
}

#[derive(Clone, PartialEq, Message)]
struct ProtoArray {
    #[prost(message, repeated, tag = "1")]
    items: Vec<ProtoValue>,
}

#[derive(Clone, PartialEq, Message)]
struct ProtoObject {
    #[prost(message, repeated, tag = "1")]
    fields: Vec<ProtoField>,
}

#[derive(Clone, PartialEq, Message)]
struct ProtoField {
    #[prost(string, tag = "1")]
    key: String,
    #[prost(message, optional, tag = "2")]
    value: Option<ProtoValue>,
}

impl ProtoValue {
    fn from_surp_value(value: &Value) -> Self {
        use proto_value::Kind;

        let kind = match value {
            Value::Null => Kind::Null(true),
            Value::Bool(value) => Kind::Bool(*value),
            Value::UInt(value) => Kind::UInt(*value),
            Value::Int(value) => Kind::Int(*value),
            Value::Float(value) => Kind::Float(*value),
            Value::Str(value) => Kind::Str(value.clone()),
            Value::Bytes(value) => Kind::Bytes(value.clone()),
            Value::Array(items) => Kind::Array(ProtoArray {
                items: items.iter().map(Self::from_surp_value).collect(),
            }),
            Value::Object(fields) => Kind::Object(ProtoObject {
                fields: fields
                    .iter()
                    .map(|(key, value)| ProtoField {
                        key: key.clone(),
                        value: Some(Self::from_surp_value(value)),
                    })
                    .collect(),
            }),
        };

        Self { kind: Some(kind) }
    }
}

fn protobuf_encode_once(value: &ProtoValue) -> Vec<u8> {
    let mut out = Vec::with_capacity(value.encoded_len());
    value.encode(&mut out).unwrap();
    out
}

// ── Surp Helpers ───────────────────────────────────────────────────

fn surp_encode_once(value: &Value) -> Vec<u8> {
    let mut enc = Encoder::new();
    enc.encode_value(value).unwrap();
    enc.finish().unwrap()
}

fn surp_encode_dedup_once(value: &Value) -> Vec<u8> {
    let mut enc = Encoder::new();
    enc.enable_dedup();
    enc.encode_value(value).unwrap();
    enc.finish().unwrap()
}

fn bench_surp_encode(ds: &Dataset, iters: usize, encoded: &[u8]) -> Measurement {
    let mut m = Measurement::new("surp", ds.name, "encode");
    m.serialized_size = Some(encoded.len());

    // Warm up.
    for _ in 0..WARMUP {
        let mut enc = Encoder::new();
        enc.encode_value(&ds.value).unwrap();
        let _ = enc.finish().unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let mut enc = Encoder::new();
        enc.encode_value(&ds.value).unwrap();
        let _ = enc.finish().unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_surp_decode(ds: &Dataset, iters: usize, encoded: &[u8]) -> Measurement {
    let mut m = Measurement::new("surp", ds.name, "decode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let mut dec = Decoder::new(encoded);
        let _ = dec.decode_next().unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let mut dec = Decoder::new(encoded);
        let _ = dec.decode_next().unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_surp_roundtrip(ds: &Dataset, iters: usize) -> Measurement {
    let mut m = Measurement::new("surp", ds.name, "roundtrip");

    for _ in 0..WARMUP {
        let mut enc = Encoder::new();
        enc.encode_value(&ds.value).unwrap();
        let bytes = enc.finish().unwrap();
        let mut dec = Decoder::new(&bytes);
        let _ = dec.decode_next().unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let mut enc = Encoder::new();
        enc.encode_value(&ds.value).unwrap();
        let bytes = enc.finish().unwrap();
        let mut dec = Decoder::new(&bytes);
        let _ = dec.decode_next().unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_surp_dedup_encode(ds: &Dataset, iters: usize, encoded: &[u8]) -> Measurement {
    let mut m = Measurement::new("surp_dedup", ds.name, "encode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let mut enc = Encoder::new();
        enc.enable_dedup();
        enc.encode_value(&ds.value).unwrap();
        let _ = enc.finish().unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let mut enc = Encoder::new();
        enc.enable_dedup();
        enc.encode_value(&ds.value).unwrap();
        let _ = enc.finish().unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_surp_dedup_decode(ds: &Dataset, iters: usize, encoded: &[u8]) -> Measurement {
    let mut m = Measurement::new("surp_dedup", ds.name, "decode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let mut dec = Decoder::new(encoded);
        let _ = dec.decode_next().unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let mut dec = Decoder::new(encoded);
        let _ = dec.decode_next().unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

// ── JSON ────────────────────────────────────────────────────────────

fn bench_json_encode(
    ds: &Dataset,
    iters: usize,
    json_val: &serde_json::Value,
    encoded: &[u8],
) -> Measurement {
    let mut m = Measurement::new("json", ds.name, "encode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let _ = serde_json::to_vec(json_val).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let _ = serde_json::to_vec(json_val).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_json_decode(ds: &Dataset, iters: usize, encoded: &[u8]) -> Measurement {
    let mut m = Measurement::new("json", ds.name, "decode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let _: serde_json::Value = serde_json::from_slice(encoded).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let _: serde_json::Value = serde_json::from_slice(encoded).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_json_roundtrip(ds: &Dataset, iters: usize, json_val: &serde_json::Value) -> Measurement {
    let mut m = Measurement::new("json", ds.name, "roundtrip");

    for _ in 0..WARMUP {
        let bytes = serde_json::to_vec(json_val).unwrap();
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let bytes = serde_json::to_vec(json_val).unwrap();
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

// ── MessagePack ─────────────────────────────────────────────────────

fn bench_msgpack_encode(
    ds: &Dataset,
    iters: usize,
    json_val: &serde_json::Value,
    encoded: &[u8],
) -> Measurement {
    let mut m = Measurement::new("msgpack", ds.name, "encode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let _ = rmp_serde::to_vec(json_val).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let _ = rmp_serde::to_vec(json_val).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_msgpack_decode(ds: &Dataset, iters: usize, encoded: &[u8]) -> Measurement {
    let mut m = Measurement::new("msgpack", ds.name, "decode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let _: serde_json::Value = rmp_serde::from_slice(encoded).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let _: serde_json::Value = rmp_serde::from_slice(encoded).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_msgpack_roundtrip(
    ds: &Dataset,
    iters: usize,
    json_val: &serde_json::Value,
) -> Measurement {
    let mut m = Measurement::new("msgpack", ds.name, "roundtrip");

    for _ in 0..WARMUP {
        let bytes = rmp_serde::to_vec(json_val).unwrap();
        let _: serde_json::Value = rmp_serde::from_slice(&bytes).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let bytes = rmp_serde::to_vec(json_val).unwrap();
        let _: serde_json::Value = rmp_serde::from_slice(&bytes).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

// ── CBOR ────────────────────────────────────────────────────────────

fn bench_cbor_encode(
    ds: &Dataset,
    iters: usize,
    json_val: &serde_json::Value,
    encoded: &[u8],
) -> Measurement {
    let mut m = Measurement::new("cbor", ds.name, "encode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let mut buf = Vec::new();
        ciborium::into_writer(json_val, &mut buf).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let mut buf = Vec::new();
        ciborium::into_writer(json_val, &mut buf).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_cbor_decode(ds: &Dataset, iters: usize, encoded: &[u8]) -> Measurement {
    let mut m = Measurement::new("cbor", ds.name, "decode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let _: serde_json::Value = ciborium::from_reader(encoded).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let _: serde_json::Value = ciborium::from_reader(encoded).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_cbor_roundtrip(ds: &Dataset, iters: usize, json_val: &serde_json::Value) -> Measurement {
    let mut m = Measurement::new("cbor", ds.name, "roundtrip");

    for _ in 0..WARMUP {
        let mut buf = Vec::new();
        ciborium::into_writer(json_val, &mut buf).unwrap();
        let _: serde_json::Value = ciborium::from_reader(buf.as_slice()).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let mut buf = Vec::new();
        ciborium::into_writer(json_val, &mut buf).unwrap();
        let _: serde_json::Value = ciborium::from_reader(buf.as_slice()).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

// -- Protocol Buffers ------------------------------------------------

fn bench_protobuf_encode(
    ds: &Dataset,
    iters: usize,
    proto_value: &ProtoValue,
    encoded: &[u8],
) -> Measurement {
    let mut m = Measurement::new("protobuf", ds.name, "encode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let _ = protobuf_encode_once(proto_value);
    }

    for _ in 0..iters {
        let start = Instant::now();
        let _ = protobuf_encode_once(proto_value);
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_protobuf_decode(ds: &Dataset, iters: usize, encoded: &[u8]) -> Measurement {
    let mut m = Measurement::new("protobuf", ds.name, "decode");
    m.serialized_size = Some(encoded.len());

    for _ in 0..WARMUP {
        let _ = ProtoValue::decode(encoded).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let _ = ProtoValue::decode(encoded).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}

fn bench_protobuf_roundtrip(ds: &Dataset, iters: usize, proto_value: &ProtoValue) -> Measurement {
    let mut m = Measurement::new("protobuf", ds.name, "roundtrip");

    for _ in 0..WARMUP {
        let bytes = protobuf_encode_once(proto_value);
        let _ = ProtoValue::decode(bytes.as_slice()).unwrap();
    }

    for _ in 0..iters {
        let start = Instant::now();
        let bytes = protobuf_encode_once(proto_value);
        let _ = ProtoValue::decode(bytes.as_slice()).unwrap();
        m.add_duration(start.elapsed());
    }
    m
}
