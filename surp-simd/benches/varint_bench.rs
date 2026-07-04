//! Criterion benchmarks comparing the scalar and SIMD-prescan varint batch
//! decoders in surp-simd.
//!
//! Run with: `cargo bench -p surp-simd --features simd-varint`
//!
//! This exists to prove that `batch_decode_varints_simd` is not doing
//! strictly more work than the plain scalar `batch_decode_varints` path
//! (see the fix for the bug where the NEON pre-scan result was discarded
//! and the scalar decoder was redundantly re-run for every varint).

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use surp_simd::{batch_decode_varints, batch_decode_varints_simd};

/// Build a realistic varint-heavy buffer: a mix of 1-, 2-, 3-, and
/// multi-byte varints, repeated many times to give a decent throughput
/// measurement and to span many 16-byte NEON scan windows.
fn make_varint_buffer(count: usize) -> (Vec<u8>, usize) {
    let pattern: &[u64] = &[
        0,
        1,
        63,
        127,   // 1 byte
        128,
        255,
        300,
        16_383, // 2 bytes
        16_384,
        1_000_000, // 3 bytes
        1 << 27,
        1 << 34,
        1 << 41, // 4-6 bytes
        u64::MAX - 1,
        u64::MAX, // 10 bytes (max length)
    ];

    let mut data = Vec::new();
    let mut n = 0usize;
    while n < count {
        for &v in pattern {
            if n >= count {
                break;
            }
            surp_core::varint::encode_varint_vec(v, &mut data);
            n += 1;
        }
    }
    (data, n)
}

fn bench_varint_batch_decode(c: &mut Criterion) {
    const COUNT: usize = 10_000;
    let (data, count) = make_varint_buffer(COUNT);

    let mut group = c.benchmark_group("varint_batch_decode");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("scalar", |b| {
        b.iter(|| {
            let results = batch_decode_varints(black_box(&data), black_box(count));
            black_box(results);
        });
    });

    group.bench_function("simd_prescan", |b| {
        b.iter(|| {
            let results = batch_decode_varints_simd(black_box(&data), black_box(count));
            black_box(results);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_varint_batch_decode);
criterion_main!(benches);
