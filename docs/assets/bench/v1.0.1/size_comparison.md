# Size Comparison

| Dataset | Surp | Surp+Dedup | JSON | MsgPack | CBOR | Protobuf | Surp/JSON |
|---------|-------|-------------|------|---------|------|----------|------------|
| small_objects | 8.6 MB | 12.0 MB | 10.5 MB | 7.8 MB | 7.9 MB | 11.4 MB | 0.82x |
| string_heavy | 1.0 MB | 668.3 KB | 1.1 MB | 925.8 KB | 927.0 KB | 1.2 MB | 0.96x |
| nested_deep | 1.0 MB | 1.5 MB | 1.2 MB | 835.1 KB | 835.3 KB | 1.4 MB | 0.87x |
| binary_blobs | 6.4 MB | 6.4 MB | 8.5 MB | 8.5 MB | 8.5 MB | 6.4 MB | 0.75x |
| mixed_api_events | 1.9 MB | 2.8 MB | 2.0 MB | 1.7 MB | 1.7 MB | 2.2 MB | 0.92x |
| numeric_heavy | 3.7 MB | 3.7 MB | 6.0 MB | 3.5 MB | 3.5 MB | 5.0 MB | 0.63x |
