# Surp Regression Benchmark Report

**Version:** `v1.0.1`
**Timestamp:** 2026-05-17T17:30:36.833184+00:00
**Mode:** full
**Dataset version:** 1.0.0

## System

| Property | Value |
|----------|-------|
| OS | macos | 
| Arch | aarch64 |
| CPU |  |
| Cores | 10 |
| RAM | 0 MB |
| Rust | rustc 1.94.1 (e408947bf 2026-03-25) (Homebrew) |

## NO REGRESSIONS DETECTED

## Performance Summary

| Format | Dataset | Op | Median (us) | p95 (us) | CV% | MB/s | Size |
|--------|---------|-----|-------------|----------|-----|------|------|
| surp | small_objects | encode | 17900.7 | 20816.4 | 6.9 | 504.6 | 8.6 MB |
| surp | small_objects | decode | 19978.2 | 22463.4 | 6.2 | 452.2 | 8.6 MB |
| surp | small_objects | roundtrip | 37460.0 | 40955.6 | 5.7 | - | - |
| surp_dedup | small_objects | encode | 72266.5 | 90686.1 | 11.8 | 173.6 | 12.0 MB |
| surp_dedup | small_objects | decode | 52247.6 | 65375.2 | 12.2 | 240.1 | 12.0 MB |
| json | small_objects | encode | 17543.2 | 20006.4 | 6.3 | 629.5 | 10.5 MB |
| json | small_objects | decode | 64717.3 | 71764.6 | 4.4 | 170.6 | 10.5 MB |
| json | small_objects | roundtrip | 85053.0 | 95081.4 | 5.3 | - | - |
| msgpack | small_objects | encode | 14512.0 | 14909.3 | 2.8 | 566.3 | 7.8 MB |
| msgpack | small_objects | decode | 61625.2 | 85137.2 | 11.4 | 133.4 | 7.8 MB |
| msgpack | small_objects | roundtrip | 67949.7 | 77353.2 | 5.8 | - | - |
| cbor | small_objects | encode | 15828.9 | 16334.9 | 1.1 | 523.8 | 7.9 MB |
| cbor | small_objects | decode | 131485.5 | 196477.2 | 18.6 | 63.1 | 7.9 MB |
| cbor | small_objects | roundtrip | 132000.1 | 140961.6 | 3.3 | - | - |
| protobuf | small_objects | encode | 25565.4 | 29562.6 | 6.7 | 466.8 | 11.4 MB |
| protobuf | small_objects | decode | 76722.0 | 92795.0 | 7.3 | 155.5 | 11.4 MB |
| protobuf | small_objects | roundtrip | 111780.7 | 140829.1 | 11.3 | - | - |
| surp | string_heavy | encode | 1067.5 | 1095.9 | 1.0 | 999.4 | 1.0 MB |
| surp | string_heavy | decode | 1906.8 | 1944.3 | 0.9 | 559.5 | 1.0 MB |
| surp | string_heavy | roundtrip | 2983.8 | 3087.8 | 1.3 | - | - |
| surp_dedup | string_heavy | encode | 4787.8 | 4910.3 | 1.3 | 142.9 | 668.3 KB |
| surp_dedup | string_heavy | decode | 3083.9 | 3188.9 | 1.5 | 221.9 | 668.3 KB |
| json | string_heavy | encode | 1207.5 | 1292.3 | 3.3 | 916.6 | 1.1 MB |
| json | string_heavy | decode | 5159.7 | 5399.6 | 1.8 | 214.5 | 1.1 MB |
| json | string_heavy | roundtrip | 6553.5 | 6770.6 | 2.2 | - | - |
| msgpack | string_heavy | encode | 1009.0 | 1716.3 | 22.5 | 939.5 | 925.8 KB |
| msgpack | string_heavy | decode | 4439.5 | 4530.6 | 0.8 | 213.5 | 925.8 KB |
| msgpack | string_heavy | roundtrip | 5422.8 | 5504.0 | 0.7 | - | - |
| cbor | string_heavy | encode | 1064.1 | 1075.6 | 0.5 | 892.1 | 927.0 KB |
| cbor | string_heavy | decode | 7587.7 | 7993.0 | 1.8 | 125.1 | 927.0 KB |
| cbor | string_heavy | roundtrip | 8760.3 | 8873.4 | 0.7 | - | - |
| protobuf | string_heavy | encode | 1984.1 | 1991.4 | 0.2 | 614.5 | 1.2 MB |
| protobuf | string_heavy | decode | 6052.4 | 6071.3 | 0.3 | 201.4 | 1.2 MB |
| protobuf | string_heavy | roundtrip | 8109.7 | 8183.4 | 0.8 | - | - |
| surp | nested_deep | encode | 2421.2 | 5110.2 | 31.4 | 451.0 | 1.0 MB |
| surp | nested_deep | decode | 4750.2 | 4797.0 | 0.4 | 229.9 | 1.0 MB |
| surp | nested_deep | roundtrip | 6474.6 | 7164.4 | 4.5 | - | - |
| surp_dedup | nested_deep | encode | 10519.8 | 10768.5 | 1.0 | 154.1 | 1.5 MB |
| surp_dedup | nested_deep | decode | 8085.8 | 9074.3 | 5.5 | 200.5 | 1.5 MB |
| json | nested_deep | encode | 2434.1 | 3282.7 | 13.8 | 516.7 | 1.2 MB |
| json | nested_deep | decode | 9935.3 | 10041.5 | 0.7 | 126.6 | 1.2 MB |
| json | nested_deep | roundtrip | 13021.8 | 14323.6 | 3.6 | - | - |
| msgpack | nested_deep | encode | 2252.0 | 2460.5 | 3.5 | 379.7 | 835.1 KB |
| msgpack | nested_deep | decode | 9082.4 | 9206.8 | 1.6 | 94.2 | 835.1 KB |
| msgpack | nested_deep | roundtrip | 11737.3 | 17500.2 | 15.3 | - | - |
| cbor | nested_deep | encode | 2836.5 | 3084.2 | 5.3 | 301.5 | 835.3 KB |
| cbor | nested_deep | decode | 14713.8 | 14825.2 | 0.8 | 58.1 | 835.3 KB |
| cbor | nested_deep | roundtrip | 17941.2 | 18480.8 | 1.2 | - | - |
| protobuf | nested_deep | encode | 32348.3 | 33534.1 | 3.9 | 46.6 | 1.4 MB |
| protobuf | nested_deep | decode | 10975.5 | 11181.8 | 1.2 | 137.2 | 1.4 MB |
| protobuf | nested_deep | roundtrip | 44348.4 | 45293.3 | 0.7 | - | - |
| surp | binary_blobs | encode | 2802.2 | 3218.8 | 14.6 | 2385.7 | 6.4 MB |
| surp | binary_blobs | decode | 430.0 | 432.5 | 0.2 | 15547.0 | 6.4 MB |
| surp | binary_blobs | roundtrip | 3617.9 | 3664.7 | 2.0 | - | - |
| surp_dedup | binary_blobs | encode | 3524.2 | 3637.1 | 7.7 | 1896.2 | 6.4 MB |
| surp_dedup | binary_blobs | decode | 433.9 | 436.5 | 0.5 | 15402.6 | 6.4 MB |
| json | binary_blobs | encode | 5164.3 | 5386.2 | 2.8 | 1725.8 | 8.5 MB |
| json | binary_blobs | decode | 1378.3 | 1426.5 | 1.2 | 6466.2 | 8.5 MB |
| json | binary_blobs | roundtrip | 7057.8 | 9748.2 | 12.0 | - | - |
| msgpack | binary_blobs | encode | 1416.8 | 1478.2 | 3.7 | 6289.9 | 8.5 MB |
| msgpack | binary_blobs | decode | 461.8 | 472.6 | 0.8 | 19295.3 | 8.5 MB |
| msgpack | binary_blobs | roundtrip | 2234.0 | 2539.4 | 4.9 | - | - |
| cbor | binary_blobs | encode | 1360.6 | 1488.8 | 5.8 | 6549.7 | 8.5 MB |
| cbor | binary_blobs | decode | 942.1 | 982.6 | 1.8 | 9458.8 | 8.5 MB |
| cbor | binary_blobs | roundtrip | 2758.4 | 3235.0 | 6.4 | - | - |
| protobuf | binary_blobs | encode | 189.8 | 194.8 | 3.4 | 35245.8 | 6.4 MB |
| protobuf | binary_blobs | decode | 418.9 | 463.0 | 3.4 | 15964.7 | 6.4 MB |
| protobuf | binary_blobs | roundtrip | 649.1 | 661.9 | 1.7 | - | - |
| surp | mixed_api_events | encode | 1742.3 | 1750.8 | 0.2 | 1120.2 | 1.9 MB |
| surp | mixed_api_events | decode | 2621.6 | 2661.9 | 0.8 | 744.5 | 1.9 MB |
| surp | mixed_api_events | roundtrip | 4372.1 | 4401.0 | 0.4 | - | - |
| surp_dedup | mixed_api_events | encode | 8257.9 | 8345.8 | 0.8 | 352.1 | 2.8 MB |
| surp_dedup | mixed_api_events | decode | 5411.0 | 5647.4 | 1.5 | 537.3 | 2.8 MB |
| json | mixed_api_events | encode | 2090.7 | 2108.8 | 0.6 | 1016.3 | 2.0 MB |
| json | mixed_api_events | decode | 7884.8 | 7939.4 | 0.4 | 269.5 | 2.0 MB |
| json | mixed_api_events | roundtrip | 9973.9 | 10089.2 | 0.6 | - | - |
| msgpack | mixed_api_events | encode | 1495.7 | 1529.7 | 1.0 | 1225.4 | 1.7 MB |
| msgpack | mixed_api_events | decode | 6750.0 | 6792.8 | 0.4 | 271.5 | 1.7 MB |
| msgpack | mixed_api_events | roundtrip | 8247.6 | 11490.8 | 11.9 | - | - |
| cbor | mixed_api_events | encode | 1827.6 | 1831.2 | 0.2 | 1002.9 | 1.7 MB |
| cbor | mixed_api_events | decode | 12641.3 | 12750.0 | 0.4 | 145.0 | 1.7 MB |
| cbor | mixed_api_events | roundtrip | 14649.7 | 19247.5 | 11.3 | - | - |
| protobuf | mixed_api_events | encode | 4224.0 | 4508.7 | 3.3 | 543.5 | 2.2 MB |
| protobuf | mixed_api_events | decode | 10061.0 | 10555.0 | 2.0 | 228.2 | 2.2 MB |
| protobuf | mixed_api_events | roundtrip | 14471.5 | 14829.1 | 2.3 | - | - |
| surp | numeric_heavy | encode | 6345.8 | 6403.5 | 2.2 | 617.8 | 3.7 MB |
| surp | numeric_heavy | decode | 7546.3 | 11363.1 | 15.8 | 519.5 | 3.7 MB |
| surp | numeric_heavy | roundtrip | 14807.7 | 15401.0 | 2.4 | - | - |
| surp_dedup | numeric_heavy | encode | 5721.8 | 6226.1 | 3.3 | 685.2 | 3.7 MB |
| surp_dedup | numeric_heavy | decode | 8164.2 | 8341.0 | 1.2 | 480.2 | 3.7 MB |
| json | numeric_heavy | encode | 8415.0 | 9486.0 | 5.0 | 743.8 | 6.0 MB |
| json | numeric_heavy | decode | 25459.8 | 25722.5 | 1.1 | 245.8 | 6.0 MB |
| json | numeric_heavy | roundtrip | 34742.8 | 34990.6 | 0.8 | - | - |
| msgpack | numeric_heavy | encode | 4879.6 | 4915.2 | 0.5 | 746.5 | 3.5 MB |
| msgpack | numeric_heavy | decode | 18946.1 | 19098.9 | 0.5 | 192.3 | 3.5 MB |
| msgpack | numeric_heavy | roundtrip | 24546.0 | 24700.8 | 0.8 | - | - |
| cbor | numeric_heavy | encode | 5652.1 | 6287.7 | 4.9 | 642.2 | 3.5 MB |
| cbor | numeric_heavy | decode | 42492.5 | 42865.5 | 4.6 | 85.4 | 3.5 MB |
| cbor | numeric_heavy | roundtrip | 49614.6 | 51327.1 | 4.5 | - | - |
| protobuf | numeric_heavy | encode | 11414.8 | 11708.9 | 4.6 | 457.3 | 5.0 MB |
| protobuf | numeric_heavy | decode | 26670.6 | 27209.5 | 2.7 | 195.7 | 5.0 MB |
| protobuf | numeric_heavy | roundtrip | 38182.5 | 39011.2 | 3.5 | - | - |


---
*Generated by surp-bench v1.0.1 on 2026-05-17T17:30:36.833184+00:00*
