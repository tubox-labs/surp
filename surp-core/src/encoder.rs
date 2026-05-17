//! Encoder for the Surp binary format.
//!
//! Encodes `Value` instances into the canonical Surp binary representation.
//! The encoder handles:
//! - Block framing with checksums
//! - Wire-type-tagged field encoding
//! - Varint/ZigZag integer encoding
//! - Length-delimited strings and bytes

use std::collections::HashMap;

use crate::checksum::compute_xxh64;
use crate::error::{Result, SurpError};
use crate::limits::Limits;
use crate::value::Value;
use crate::varint::encode_varint_vec;
use crate::wire::{BlockType, CompressionType, WireType};

/// Encoder that serializes `Value`s into Surp binary format.
///
/// # Example
/// ```
/// use surp_core::{Encoder, Value};
///
/// let mut enc = Encoder::new();
/// enc.encode_value(&Value::UInt(42)).unwrap();
/// let bytes = enc.finish().unwrap();
/// assert!(!bytes.is_empty());
/// ```
pub struct Encoder {
    /// The output buffer accumulating the binary output.
    output: Vec<u8>,
    /// Buffer for the current block's payload (before framing).
    block_buf: Vec<u8>,
    /// Current nesting depth (for overflow protection).
    depth: usize,
    /// Resource limits.
    limits: Limits,
    /// Compression type for blocks.
    compression: CompressionType,
    /// Per-block string dictionary: string → index.
    /// When `dedup_strings` is true, repeated strings are encoded as Reference.
    string_dict: HashMap<String, u32>,
    /// Whether to enable string deduplication.
    dedup_strings: bool,
}

impl Encoder {
    /// Create a new encoder with default settings.
    pub fn new() -> Self {
        Self {
            output: Vec::with_capacity(4096),
            block_buf: Vec::with_capacity(4096),
            depth: 0,
            limits: Limits::default(),
            compression: CompressionType::None,
            string_dict: HashMap::new(),
            dedup_strings: false,
        }
    }

    /// Create an encoder with custom limits.
    pub fn with_limits(limits: Limits) -> Self {
        Self {
            limits,
            ..Self::new()
        }
    }

    /// Create an encoder with a size hint for pre-allocation.
    ///
    /// Use this when you have an estimate of the final encoded size to avoid
    /// reallocations during encoding. The `estimated_size` should be an estimate
    /// of the total output size in bytes.
    ///
    /// # Example
    /// ```
    /// use surp_core::Encoder;
    ///
    /// // Pre-allocate for ~10KB of output
    /// let mut enc = Encoder::with_size_hint(10_000);
    /// ```
    pub fn with_size_hint(estimated_size: usize) -> Self {
        // Add some headroom for block framing overhead (header + checksum + length fields)
        let capacity = estimated_size.saturating_add(128);
        Self {
            output: Vec::with_capacity(capacity),
            block_buf: Vec::with_capacity(estimated_size),
            depth: 0,
            limits: Limits::default(),
            compression: CompressionType::None,
            string_dict: HashMap::new(),
            dedup_strings: false,
        }
    }

    /// Enable string deduplication. Repeated strings within a block
    /// will be encoded as Reference wire types pointing to the dictionary.
    pub fn enable_dedup(&mut self) {
        self.dedup_strings = true;
    }

    /// Set the compression type for subsequent blocks.
    pub fn set_compression(&mut self, comp: CompressionType) {
        self.compression = comp;
    }

    /// Encode a single `Value` into the current block buffer.
    ///
    /// This is the main entry point for encoding. Values are accumulated
    /// in the block buffer; call `finish()` to flush and produce the final bytes.
    pub fn encode_value(&mut self, value: &Value) -> Result<()> {
        self.encode_value_inner(value)
    }

    fn encode_value_inner(&mut self, value: &Value) -> Result<()> {
        match value {
            Value::Null => {
                self.block_buf.push(WireType::Null.to_tag());
            }
            Value::Bool(b) => {
                self.block_buf.push(WireType::Bool.to_tag());
                self.block_buf.push(if *b { 0x01 } else { 0x00 });
            }
            Value::UInt(n) => {
                self.block_buf.push(WireType::VarUInt.to_tag());
                encode_varint_vec(*n, &mut self.block_buf);
            }
            Value::Int(n) => {
                self.block_buf.push(WireType::VarInt.to_tag());
                crate::varint::encode_signed_varint_vec(*n, &mut self.block_buf);
            }
            Value::Float(f) => {
                self.block_buf.push(WireType::Fixed64.to_tag());
                self.block_buf.extend_from_slice(&f.to_le_bytes());
            }
            Value::Str(s) => {
                if self.dedup_strings {
                    if let Some(&idx) = self.string_dict.get(s.as_str()) {
                        // Emit a Reference to the dictionary entry.
                        self.block_buf.push(WireType::Reference.to_tag());
                        encode_varint_vec(idx as u64, &mut self.block_buf);
                        return Ok(());
                    }
                    // First occurrence: record in dictionary.
                    let idx = self.string_dict.len() as u32;
                    self.string_dict.insert(s.clone(), idx);
                }
                self.block_buf.push(WireType::LenDelimited.to_tag());
                // Sub-type marker: 0x00 = UTF-8 string
                self.block_buf.push(0x00);
                encode_varint_vec(s.len() as u64, &mut self.block_buf);
                self.block_buf.extend_from_slice(s.as_bytes());
            }
            Value::Bytes(b) => {
                self.block_buf.push(WireType::LenDelimited.to_tag());
                // Sub-type marker: 0x01 = raw binary
                self.block_buf.push(0x01);
                encode_varint_vec(b.len() as u64, &mut self.block_buf);
                self.block_buf.extend_from_slice(b);
            }
            Value::Array(items) => {
                if self.depth >= self.limits.max_nesting_depth {
                    return Err(SurpError::NestingTooDeep(
                        self.depth,
                        self.limits.max_nesting_depth,
                    ));
                }
                if items.len() > self.limits.max_items {
                    return Err(SurpError::TooManyItems(items.len(), self.limits.max_items));
                }
                self.block_buf.push(WireType::StartArray.to_tag());
                // Encode item count as a varint for fast skipping.
                encode_varint_vec(items.len() as u64, &mut self.block_buf);
                self.depth += 1;
                for item in items {
                    self.encode_value_inner(item)?;
                }
                self.depth -= 1;
                self.block_buf.push(WireType::EndArray.to_tag());
            }
            Value::Object(entries) => {
                if self.depth >= self.limits.max_nesting_depth {
                    return Err(SurpError::NestingTooDeep(
                        self.depth,
                        self.limits.max_nesting_depth,
                    ));
                }
                if entries.len() > self.limits.max_items {
                    return Err(SurpError::TooManyItems(
                        entries.len(),
                        self.limits.max_items,
                    ));
                }
                self.block_buf.push(WireType::StartObject.to_tag());
                // Encode entry count for fast skipping.
                encode_varint_vec(entries.len() as u64, &mut self.block_buf);
                self.depth += 1;
                for (key, val) in entries {
                    // Encode key as a length-delimited string inline.
                    encode_varint_vec(key.len() as u64, &mut self.block_buf);
                    self.block_buf.extend_from_slice(key.as_bytes());
                    // Encode value.
                    self.encode_value_inner(val)?;
                }
                self.depth -= 1;
                self.block_buf.push(WireType::EndObject.to_tag());
            }
        }
        Ok(())
    }

    /// Flush the current block buffer into a framed block and append to output.
    /// Returns the number of bytes in the flushed block.
    ///
    /// When `dedup_strings` is enabled and the per-block string dictionary is
    /// non-empty, a `StringDict` block is emitted *before* the data block.
    /// The dictionary entries are sorted and stored using prefix-delta
    /// compression for compactness.
    ///
    /// When `compression` is set to something other than `None`, the block
    /// payload is compressed before framing. The checksum is always computed
    /// on the **uncompressed** payload so the decoder can verify integrity
    /// after decompression. The block_len field reflects the **compressed**
    /// size written to the wire.
    pub fn flush_block(&mut self) -> Result<usize> {
        if self.block_buf.is_empty() {
            return Ok(0);
        }

        let mut total_size = 0;

        // --- Emit StringDict block before the data block ---
        if self.dedup_strings && !self.string_dict.is_empty() {
            let dict_payload = self.encode_string_dict_payload();
            let dict_checksum = compute_xxh64(&dict_payload);

            self.output.push(BlockType::StringDict as u8);
            encode_varint_vec(dict_payload.len() as u64, &mut self.output);
            self.output.push(CompressionType::None as u8);
            self.output.extend_from_slice(&dict_checksum.to_le_bytes());
            self.output.extend_from_slice(&dict_payload);

            total_size += 1 + 1 + 1 + 8 + dict_payload.len();
        }

        // --- Emit the data block ---

        // Checksum is always over the uncompressed payload.
        let checksum = compute_xxh64(&self.block_buf);

        // Compress if requested. The on-wire payload may differ from block_buf.
        // Optimization: use std::mem::take to avoid cloning when possible.
        let (wire_payload, wire_comp) = if self.compression != CompressionType::None {
            match self.compress_payload(&self.block_buf) {
                Some(compressed)
                    if compressed.len() < self.block_buf.len()
                        && !compressed.is_empty()
                        && (self.block_buf.len() / compressed.len())
                            <= self.limits.max_decompression_ratio =>
                {
                    // Store uncompressed length as a varint prefix so the decoder
                    // can pre-allocate the decompression buffer.
                    let mut framed = Vec::with_capacity(10 + compressed.len());
                    encode_varint_vec(self.block_buf.len() as u64, &mut framed);
                    framed.extend_from_slice(&compressed);
                    // Clear block_buf since we're using the compressed version
                    self.block_buf.clear();
                    (framed, self.compression)
                }
                _ => {
                    // Compression didn't help — store uncompressed.
                    // Use take() instead of clone() to avoid allocation.
                    (std::mem::take(&mut self.block_buf), CompressionType::None)
                }
            }
        } else {
            // No compression — take ownership instead of cloning.
            (std::mem::take(&mut self.block_buf), CompressionType::None)
        };

        // Block header:
        //   block_type (1B) | block_len (varint) | comp_type (1B) | checksum (8B) | payload
        let block_type = BlockType::Data as u8;

        self.output.push(block_type);
        encode_varint_vec(wire_payload.len() as u64, &mut self.output);
        self.output.push(wire_comp as u8);
        self.output.extend_from_slice(&checksum.to_le_bytes());
        self.output.extend_from_slice(&wire_payload);

        total_size += 1 + 1 + 8 + wire_payload.len();
        // block_buf is already cleared by take() or explicit clear() above
        self.string_dict.clear(); // Reset per-block dictionary.
        Ok(total_size)
    }

    /// Encode the per-block string dictionary as a prefix-delta-compressed
    /// payload for a `StringDict` block.
    ///
    /// Layout:
    ///   `entry_count(varint)` | entries...
    ///
    /// Each entry (prefix-delta encoded):
    ///   `original_index(varint)` | `prefix_len(varint)` | `suffix_len(varint)` | `suffix_bytes`
    ///
    /// Entries are sorted lexicographically for prefix sharing. The
    /// `original_index` preserves the insertion order so the decoder can
    /// rebuild the reference table with the correct indices.
    fn encode_string_dict_payload(&self) -> Vec<u8> {
        // Collect entries and sort by string for prefix-delta compression.
        let mut entries: Vec<(&str, u32)> = self
            .string_dict
            .iter()
            .map(|(s, &idx)| (s.as_str(), idx))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        let mut payload = Vec::with_capacity(entries.len() * 16);
        encode_varint_vec(entries.len() as u64, &mut payload);

        let mut prev = "";
        for (s, original_idx) in &entries {
            // Compute shared prefix length with previous entry.
            let prefix_len = s
                .as_bytes()
                .iter()
                .zip(prev.as_bytes().iter())
                .take_while(|(a, b)| a == b)
                .count();
            let suffix = &s.as_bytes()[prefix_len..];

            encode_varint_vec(*original_idx as u64, &mut payload);
            encode_varint_vec(prefix_len as u64, &mut payload);
            encode_varint_vec(suffix.len() as u64, &mut payload);
            payload.extend_from_slice(suffix);

            prev = s;
        }
        payload
    }

    /// Compress the payload using the configured compression algorithm.
    /// Returns `None` if the compression feature is not available.
    #[allow(unused_variables)]
    fn compress_payload(&self, data: &[u8]) -> Option<Vec<u8>> {
        match self.compression {
            CompressionType::None => None,
            #[cfg(feature = "zstd")]
            CompressionType::Zstd => zstd::encode_all(std::io::Cursor::new(data), 3).ok(),
            #[cfg(not(feature = "zstd"))]
            CompressionType::Zstd => None,
            #[cfg(feature = "snappy")]
            CompressionType::Snappy => {
                let mut enc = snap::raw::Encoder::new();
                enc.compress_vec(data).ok()
            }
            #[cfg(not(feature = "snappy"))]
            CompressionType::Snappy => None,
            #[cfg(feature = "lz4")]
            CompressionType::Lz4 => Some(lz4_flex::compress_prepend_size(data)),
            #[cfg(not(feature = "lz4"))]
            CompressionType::Lz4 => None,
        }
    }

    /// Finish encoding: flush remaining data and return the complete binary output.
    ///
    /// The output includes data blocks + file trailer checksum.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        self.flush_block()?;

        // Write file trailer: XXH64 checksum over everything written so far.
        let overall_checksum = compute_xxh64(&self.output);
        // Trailer block: type=0xFF, length=8, no compression, checksum of checksum, payload=checksum
        self.output.push(BlockType::Trailer as u8);
        encode_varint_vec(8, &mut self.output);
        self.output.push(CompressionType::None as u8);
        let trailer_checksum = compute_xxh64(&overall_checksum.to_le_bytes());
        self.output
            .extend_from_slice(&trailer_checksum.to_le_bytes());
        self.output
            .extend_from_slice(&overall_checksum.to_le_bytes());

        Ok(self.output)
    }

    /// Get the current size of the output buffer (including unflushed block data).
    pub fn current_size(&self) -> usize {
        self.output.len() + self.block_buf.len()
    }

    /// Get access to the raw block buffer (for testing/inspection).
    pub fn block_buffer(&self) -> &[u8] {
        &self.block_buf
    }
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_null() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::Null).unwrap();
        assert_eq!(enc.block_buffer(), &[0x00]); // WireType::Null
    }

    #[test]
    fn encode_bool() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::Bool(true)).unwrap();
        assert_eq!(enc.block_buffer(), &[0x01, 0x01]);
        enc.block_buf.clear();
        enc.encode_value(&Value::Bool(false)).unwrap();
        assert_eq!(enc.block_buffer(), &[0x01, 0x00]);
    }

    #[test]
    fn encode_uint_small() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::UInt(42)).unwrap();
        assert_eq!(enc.block_buffer(), &[0x02, 42]); // WireType::VarUInt, 42
    }

    #[test]
    fn encode_uint_large() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::UInt(300)).unwrap();
        assert_eq!(enc.block_buffer(), &[0x02, 0xac, 0x02]);
    }

    #[test]
    fn encode_int_negative() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::Int(-1)).unwrap();
        // ZigZag(-1) = 1, LEB128(1) = 0x01
        assert_eq!(enc.block_buffer(), &[0x03, 0x01]);
    }

    #[test]
    fn encode_float() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::Float(3.125)).unwrap();
        let mut expected = vec![0x04];
        expected.extend_from_slice(&3.125f64.to_le_bytes());
        assert_eq!(enc.block_buffer(), &expected);
    }

    #[test]
    fn encode_string() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::Str("hello".into())).unwrap();
        // WireType::LenDelimited (0x05) + sub-type 0x00 + length 5 + "hello"
        let mut expected = vec![0x05, 0x00, 5];
        expected.extend_from_slice(b"hello");
        assert_eq!(enc.block_buffer(), &expected);
    }

    #[test]
    fn encode_bytes() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::Bytes(vec![0xDE, 0xAD])).unwrap();
        // WireType::LenDelimited (0x05) + sub-type 0x01 + length 2 + bytes
        assert_eq!(enc.block_buffer(), &[0x05, 0x01, 2, 0xDE, 0xAD]);
    }

    #[test]
    fn encode_array() {
        let mut enc = Encoder::new();
        let arr = Value::Array(vec![Value::UInt(1), Value::UInt(2)]);
        enc.encode_value(&arr).unwrap();
        // StartArray(0x08) + count(2) + UInt(1) + UInt(2) + EndArray(0x09)
        assert_eq!(
            enc.block_buffer(),
            &[0x08, 0x02, 0x02, 0x01, 0x02, 0x02, 0x09]
        );
    }

    #[test]
    fn encode_object() {
        let mut enc = Encoder::new();
        let obj = Value::Object(vec![("x".into(), Value::UInt(10))]);
        enc.encode_value(&obj).unwrap();
        // StartObject(0x06) + count(1) + key_len(1) + "x" + UInt(10) + EndObject(0x07)
        assert_eq!(
            enc.block_buffer(),
            &[0x06, 0x01, 0x01, b'x', 0x02, 0x0a, 0x07]
        );
    }

    #[test]
    fn finish_produces_valid_file() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::Null).unwrap();
        let bytes = enc.finish().unwrap();
        assert_eq!(bytes[0], BlockType::Data as u8);
        // Trailer block is 19 bytes: type(1) + varint(1) + comp(1) + checksum(8) + payload(8)
        assert_eq!(bytes[bytes.len() - 19], BlockType::Trailer as u8);
    }

    #[test]
    fn nesting_depth_limit() {
        let mut enc = Encoder::with_limits(Limits {
            max_nesting_depth: 2,
            ..Limits::default()
        });
        // Nest 3 levels deep — should fail
        let val = Value::Array(vec![Value::Array(vec![Value::Array(vec![])])]);
        assert!(enc.encode_value(&val).is_err());
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn compression_ratio_guard_falls_back_to_uncompressed() {
        let mut enc = Encoder::new();
        enc.set_compression(CompressionType::Lz4);
        enc.encode_value(&Value::Str("A".repeat(20_000))).unwrap();
        let bytes = enc.finish().unwrap();

        let (block, _) =
            crate::block::BlockReader::parse(&bytes, 0).expect("data block should parse");
        assert_eq!(block.block_type, BlockType::Data);
        assert_eq!(block.compression, CompressionType::None);

        let mut dec = crate::decoder::Decoder::new(&bytes);
        let values = dec.decode_all_owned().expect("decode should succeed");
        assert_eq!(values.len(), 1);
    }
}
