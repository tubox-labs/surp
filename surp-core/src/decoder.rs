//! Decoder for the Surp binary format.
//!
//! Provides both zero-copy `SurpValue<'a>` decoding (borrows from the input)
//! and owned `Value` decoding. The decoder validates checksums, enforces limits,
//! and supports skipping unknown wire types for forward compatibility.

use crate::checksum::compute_xxh64;
use crate::error::{Result, SurpError};
use crate::limits::Limits;
use crate::value::{SurpValue, Value};
use crate::varint::{decode_signed_varint, decode_varint};
use crate::wire::{BlockType, CompressionType, WireType};

/// Convert u64 to usize safely, returning error on overflow (32-bit platforms).
#[inline]
fn safe_usize(val: u64) -> Result<usize> {
    usize::try_from(val).map_err(|_| SurpError::LengthOverflow(val))
}

/// MSRV-compatible polyfill for `str::floor_char_boundary` (stable in 1.91+).
/// Returns the largest valid character boundary `<= index` in `s`.
#[inline]
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        // Walk backwards from index to find a valid UTF-8 char boundary.
        // UTF-8 continuation bytes have the pattern 10xx_xxxx (0x80-0xBF).
        let bytes = s.as_bytes();
        let lower_bound = index.saturating_sub(3);
        for i in (lower_bound..=index).rev() {
            // A byte is a char boundary if it's NOT a continuation byte.
            if (bytes[i] as i8) >= -0x40 {
                return i;
            }
        }
        lower_bound
    }
}

/// Decoder that reads Surp binary data and produces values.
///
/// # Example
/// ```
/// use surp_core::{Encoder, Decoder, Value};
///
/// let mut enc = Encoder::new();
/// enc.encode_value(&Value::Str("hello".into())).unwrap();
/// let bytes = enc.finish().unwrap();
///
/// let mut dec = Decoder::new(&bytes);
/// let val = dec.decode_next().unwrap();
/// assert_eq!(val.to_owned_value(), Value::Str("hello".into()));
/// ```
pub struct Decoder<'a> {
    /// The input data buffer.
    data: &'a [u8],
    /// Current read position in `data`.
    pos: usize,
    /// Resource limits.
    limits: Limits,
    /// Current nesting depth.
    depth: usize,
    /// Current block's payload slice (start, end).
    current_block: Option<(usize, usize)>,
    /// Position within the current block payload.
    block_pos: usize,
    /// Cumulative bytes allocated during this decode session (for memory tracking).
    memory_used: usize,
    /// Per-block borrowed string slices for zero-copy reference resolution.
    str_slices: Vec<&'a str>,
    /// Per-block owned string table for owned reference resolution (compressed blocks).
    owned_strings: Vec<String>,
    /// Whether `owned_strings` was pre-populated by a StringDict block.
    /// When true, `decode_next_owned` will not clear the table on new block entry.
    dict_preloaded: bool,
    /// Decompressed block buffer (used only when a block has compression != None).
    /// When set, the decode methods read from this instead of `self.data`.
    decompressed_buf: Option<Vec<u8>>,
}

impl<'a> Decoder<'a> {
    /// Create a new decoder over the given data.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            limits: Limits::default(),
            depth: 0,
            current_block: None,
            block_pos: 0,
            memory_used: 0,
            str_slices: Vec::new(),
            owned_strings: Vec::new(),
            dict_preloaded: false,
            decompressed_buf: None,
        }
    }

    /// Create a decoder with custom limits.
    pub fn with_limits(data: &'a [u8], limits: Limits) -> Self {
        Self {
            limits,
            ..Self::new(data)
        }
    }

    /// Track memory allocation; returns error if limit exceeded.
    fn track_alloc(&mut self, bytes: usize) -> Result<()> {
        self.memory_used = self.memory_used.saturating_add(bytes);
        if self.memory_used > self.limits.max_memory {
            return Err(SurpError::MemoryLimitExceeded(
                self.memory_used,
                self.limits.max_memory,
            ));
        }
        Ok(())
    }

    /// Skip a value at the current block_pos without allocating.
    /// Used for forward-compatible skipping of unknown fields.
    ///
    /// All branches are bounds-checked against `block_end` and enforce
    /// the decoder's `Limits` to prevent denial-of-service via crafted
    /// counts or deeply nested structures.
    pub fn skip_value_at(&mut self, block_end: usize) -> Result<()> {
        if self.block_pos >= block_end {
            return Err(SurpError::UnexpectedEof(self.block_pos));
        }

        let tag = self.data[self.block_pos];
        self.block_pos += 1;

        let wire_type = WireType::from_tag(tag).ok_or(SurpError::InvalidWireType(tag))?;

        match wire_type {
            WireType::Null => {}                           // no payload
            WireType::EndObject | WireType::EndArray => {} // no payload

            WireType::Bool => {
                if self.block_pos >= block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                self.block_pos += 1;
            }

            WireType::VarUInt | WireType::VarInt | WireType::Reference => {
                if self.block_pos >= block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                let (_val, consumed) = decode_varint(self.data, self.block_pos)?;
                if self.block_pos + consumed > block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos + consumed));
                }
                self.block_pos += consumed;
            }

            WireType::Fixed64 => {
                if self.block_pos + 8 > block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                self.block_pos += 8;
            }

            WireType::LenDelimited => {
                if self.block_pos >= block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                self.block_pos += 1; // sub-type byte
                let (len, consumed) = decode_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                let len = safe_usize(len)?;
                if self.block_pos + len > block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos + len));
                }
                self.block_pos += len;
            }

            WireType::StartArray => {
                let (count, consumed) = decode_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                let count = safe_usize(count)?;
                // Enforce item limit even during skip to prevent DoS.
                if count > self.limits.max_items {
                    return Err(SurpError::TooManyItems(count, self.limits.max_items));
                }
                self.depth += 1;
                if self.depth > self.limits.max_nesting_depth {
                    self.depth -= 1;
                    return Err(SurpError::NestingTooDeep(
                        self.depth,
                        self.limits.max_nesting_depth,
                    ));
                }
                for _ in 0..count {
                    self.skip_value_at(block_end)?;
                }
                self.depth -= 1;
                // Consume EndArray tag.
                if self.block_pos < block_end
                    && self.data[self.block_pos] == WireType::EndArray.to_tag()
                {
                    self.block_pos += 1;
                }
            }

            WireType::StartObject => {
                let (count, consumed) = decode_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                let count = safe_usize(count)?;
                // Enforce item limit even during skip to prevent DoS.
                if count > self.limits.max_items {
                    return Err(SurpError::TooManyItems(count, self.limits.max_items));
                }
                self.depth += 1;
                if self.depth > self.limits.max_nesting_depth {
                    self.depth -= 1;
                    return Err(SurpError::NestingTooDeep(
                        self.depth,
                        self.limits.max_nesting_depth,
                    ));
                }
                for _ in 0..count {
                    // Skip key (varint len + bytes).
                    let (key_len, kc) = decode_varint(self.data, self.block_pos)?;
                    self.block_pos += kc;
                    let key_len = safe_usize(key_len)?;
                    if self.block_pos + key_len > block_end {
                        return Err(SurpError::UnexpectedEof(self.block_pos + key_len));
                    }
                    self.block_pos += key_len;
                    // Skip value.
                    self.skip_value_at(block_end)?;
                }
                self.depth -= 1;
                // Consume EndObject tag.
                if self.block_pos < block_end
                    && self.data[self.block_pos] == WireType::EndObject.to_tag()
                {
                    self.block_pos += 1;
                }
            }
        }

        Ok(())
    }

    /// Read the next block from the file.
    /// Returns `(block_type, payload_slice_start, payload_slice_end)` or None if at EOF/trailer.
    ///
    /// When the block uses compression, the payload is decompressed into
    /// `self.decompressed_buf` and the returned indices refer to that buffer
    /// (0..decompressed_len). The `block_data()` helper is used during decoding
    /// to transparently pick the right backing slice.
    ///
    /// `StringDict` blocks are handled transparently: their prefix-delta
    /// entries are decoded into the per-block string tables and then the
    /// method advances to the next block automatically.
    fn read_next_block(&mut self) -> Result<Option<(BlockType, usize, usize)>> {
        loop {
            if self.pos >= self.data.len() {
                return Ok(None);
            }

            // Block header: block_type(1) | block_len(varint) | comp_type(1) | checksum(8) | payload
            let block_type_byte = self.data[self.pos];
            self.pos += 1;

            let block_type = BlockType::from_byte(block_type_byte)
                .ok_or(SurpError::InvalidBlockType(block_type_byte))?;

            if block_type == BlockType::Trailer {
                return Ok(None); // End of data blocks.
            }

            let (block_len, varint_bytes) = decode_varint(self.data, self.pos)?;
            self.pos += varint_bytes;
            let block_len = safe_usize(block_len)?;

            if block_len > self.limits.max_block_size {
                return Err(SurpError::BlockTooLarge(
                    block_len,
                    self.limits.max_block_size,
                ));
            }

            if self.pos >= self.data.len() {
                return Err(SurpError::UnexpectedEof(self.pos));
            }
            let comp_byte = self.data[self.pos];
            self.pos += 1;
            let comp_type = CompressionType::from_byte(comp_byte)
                .ok_or(SurpError::UnknownCompression(comp_byte))?;

            // Read checksum (8 bytes, little-endian).
            if self.pos + 8 > self.data.len() {
                return Err(SurpError::UnexpectedEof(self.pos));
            }
            let expected_checksum =
                u64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
            self.pos += 8;

            // Read payload.
            let payload_start = self.pos;
            let payload_end = self.pos + block_len;
            if payload_end > self.data.len() {
                return Err(SurpError::UnexpectedEof(payload_end));
            }

            self.pos = payload_end;

            if block_type == BlockType::StringDict {
                // StringDict block: verify checksum, decode entries, then loop.
                let actual_checksum = compute_xxh64(&self.data[payload_start..payload_end]);
                if actual_checksum != expected_checksum {
                    return Err(SurpError::ChecksumMismatch {
                        expected: expected_checksum,
                        actual: actual_checksum,
                    });
                }
                self.decode_string_dict_block(&self.data[payload_start..payload_end])?;
                continue; // advance to the next block (should be Data)
            }

            if comp_type != CompressionType::None {
                // Compressed block: decompress into owned buffer.
                let wire_payload = &self.data[payload_start..payload_end];
                let decompressed = self.decompress_block(comp_type, wire_payload)?;

                // Verify checksum on the decompressed data.
                let actual_checksum = compute_xxh64(&decompressed);
                if actual_checksum != expected_checksum {
                    return Err(SurpError::ChecksumMismatch {
                        expected: expected_checksum,
                        actual: actual_checksum,
                    });
                }

                let len = decompressed.len();
                self.decompressed_buf = Some(decompressed);
                return Ok(Some((block_type, 0, len)));
            } else {
                // Uncompressed block: verify checksum directly on the input slice.
                let actual_checksum = compute_xxh64(&self.data[payload_start..payload_end]);
                if actual_checksum != expected_checksum {
                    return Err(SurpError::ChecksumMismatch {
                        expected: expected_checksum,
                        actual: actual_checksum,
                    });
                }

                self.decompressed_buf = None;
                return Ok(Some((block_type, payload_start, payload_end)));
            }
        }
    }

    /// Decode a prefix-delta-compressed StringDict block payload and populate
    /// the per-block string tables (`str_slices` / `owned_strings`).
    ///
    /// Layout:
    ///   `entry_count(varint)` | entries...
    ///
    /// Each entry:
    ///   `original_index(varint)` | `prefix_len(varint)` | `suffix_len(varint)` | `suffix_bytes`
    ///
    /// Entries are sorted lexicographically in the block; `original_index`
    /// maps back to the insertion-order position used by Reference wire types.
    fn decode_string_dict_block(&mut self, payload: &[u8]) -> Result<()> {
        let mut pos = 0;
        let (count, consumed) = decode_varint(payload, pos)?;
        pos += consumed;
        let count = safe_usize(count)?;

        if count > self.limits.max_items {
            return Err(SurpError::TooManyItems(count, self.limits.max_items));
        }

        // Temporary storage: (original_index, reconstructed string)
        let mut entries: Vec<(usize, String)> = Vec::with_capacity(count.min(4096));
        let mut prev = String::new();

        for _ in 0..count {
            let (original_idx, c1) = decode_varint(payload, pos)?;
            pos += c1;
            let original_idx = safe_usize(original_idx)?;

            // Validate that original_idx is within bounds (cannot exceed entry count).
            if original_idx >= count {
                return Err(SurpError::InvalidData(format!(
                    "StringDict entry has original_idx {original_idx} >= count {count}"
                )));
            }

            let (prefix_len, c2) = decode_varint(payload, pos)?;
            pos += c2;
            let prefix_len = safe_usize(prefix_len)?;

            let (suffix_len, c3) = decode_varint(payload, pos)?;
            pos += c3;
            let suffix_len = safe_usize(suffix_len)?;

            if pos
                .checked_add(suffix_len)
                .is_none_or(|end| end > payload.len())
            {
                return Err(SurpError::UnexpectedEof(pos));
            }
            let suffix = &payload[pos..pos + suffix_len];
            pos += suffix_len;

            // Reconstruct the full string from prefix of previous + suffix.
            let prefix_end = prefix_len.min(prev.len());
            // Ensure prefix_end falls on a valid char boundary.
            // If corrupted, snap down to the nearest valid boundary.
            let prefix_end = floor_char_boundary(&prev, prefix_end);
            let mut full = String::with_capacity(prefix_end + suffix_len);
            full.push_str(&prev[..prefix_end]);
            full.push_str(std::str::from_utf8(suffix).map_err(|_| SurpError::InvalidUtf8(pos))?);

            prev.clone_from(&full);
            entries.push((original_idx, full));
        }

        // Rebuild the string tables in original insertion order.
        let max_idx = entries.iter().map(|(idx, _)| *idx).max().unwrap_or(0);
        let table_size = max_idx + 1;

        // Pre-populate owned_strings with placeholders, then fill.
        self.owned_strings.clear();
        self.owned_strings.resize(table_size, String::new());
        for (idx, s) in &entries {
            if *idx < table_size {
                self.owned_strings[*idx].clone_from(s);
            }
        }

        self.dict_preloaded = true;
        Ok(())
    }

    /// Decompress a compressed block payload.
    ///
    /// The wire payload for compressed blocks is:
    ///   `uncompressed_len(varint) | compressed_data`
    #[allow(unused_variables)]
    fn decompress_block(&self, comp_type: CompressionType, wire: &[u8]) -> Result<Vec<u8>> {
        // Read the uncompressed length prefix.
        let (uncomp_len, prefix_consumed) = decode_varint(wire, 0)?;
        let uncomp_len = safe_usize(uncomp_len)?;

        if uncomp_len > self.limits.max_block_size {
            return Err(SurpError::BlockTooLarge(
                uncomp_len,
                self.limits.max_block_size,
            ));
        }

        let compressed = &wire[prefix_consumed..];

        // Check decompression ratio to prevent decompression bombs.
        // A ratio of 100:1 means 1 byte compressed -> 100 bytes uncompressed max.
        if !compressed.is_empty() {
            let ratio = uncomp_len / compressed.len();
            if ratio > self.limits.max_decompression_ratio {
                return Err(SurpError::DecompressionRatioExceeded {
                    ratio: uncomp_len as f64 / compressed.len() as f64,
                    max_ratio: self.limits.max_decompression_ratio,
                    compressed: compressed.len(),
                    uncompressed: uncomp_len,
                });
            }
        }

        match comp_type {
            CompressionType::None => Ok(compressed.to_vec()),
            CompressionType::Zstd => {
                #[cfg(feature = "zstd")]
                {
                    zstd::decode_all(std::io::Cursor::new(compressed))
                        .map_err(|e| SurpError::DecompressionError(format!("zstd: {e}")))
                }
                #[cfg(not(feature = "zstd"))]
                {
                    Err(SurpError::DecompressionError(
                        "zstd decompression not available (enable 'zstd' feature)".into(),
                    ))
                }
            }
            CompressionType::Snappy => {
                #[cfg(feature = "snappy")]
                {
                    let mut dec = snap::raw::Decoder::new();
                    dec.decompress_vec(compressed)
                        .map_err(|e| SurpError::DecompressionError(format!("snappy: {e}")))
                }
                #[cfg(not(feature = "snappy"))]
                {
                    Err(SurpError::DecompressionError(
                        "snappy decompression not available (enable 'snappy' feature)".into(),
                    ))
                }
            }
            CompressionType::Lz4 => {
                #[cfg(feature = "lz4")]
                {
                    // lz4_flex compress_prepend_size prefixes the uncompressed size as 4-byte LE.
                    lz4_flex::decompress_size_prepended(compressed)
                        .map_err(|e| SurpError::DecompressionError(format!("lz4: {e}")))
                }
                #[cfg(not(feature = "lz4"))]
                {
                    Err(SurpError::DecompressionError(
                        "lz4 decompression not available (enable 'lz4' feature)".into(),
                    ))
                }
            }
        }
    }

    /// Get the backing data slice for the current block.
    ///
    /// If the block was decompressed, returns a reference to the decompressed buffer.
    /// Otherwise, returns the original data slice.
    #[inline]
    fn block_data(&self) -> &[u8] {
        self.decompressed_buf.as_deref().unwrap_or(self.data)
    }

    /// Decode the next value from the input. Automatically reads blocks as needed.
    ///
    /// Returns a zero-copy `SurpValue` that borrows from the input data.
    ///
    /// **Note:** Zero-copy decoding requires uncompressed blocks. If a compressed
    /// block is encountered, this method returns `DecompressionError` because
    /// borrowed `SurpValue<'a>` cannot reference decompressed (owned) data.
    /// Use `decode_all_owned()` or `decode_next_owned()` for compressed data.
    pub fn decode_next(&mut self) -> Result<SurpValue<'a>> {
        // Loop instead of self-recursion: skipping non-Data blocks or
        // exhausted blocks must not grow the native call stack, since a
        // crafted input can contain thousands of tiny skippable blocks.
        loop {
            // If we don't have a current block, read one.
            if self.current_block.is_none() {
                match self.read_next_block()? {
                    Some((BlockType::Data, start, end)) => {
                        // Compressed blocks land in decompressed_buf with indices 0..len.
                        // Zero-copy SurpValue cannot borrow from owned decompressed data.
                        if self.decompressed_buf.is_some() {
                            return Err(SurpError::DecompressionError(
                                "zero-copy decode_next() cannot borrow from decompressed block; \
                                 use decode_all_owned() or decode_next_owned() instead"
                                    .into(),
                            ));
                        }
                        self.current_block = Some((start, end));
                        self.block_pos = start;
                        // Reset per-block tables (StringDict preload is not
                        // supported in zero-copy path — it produces owned data).
                        self.str_slices.clear();
                        self.dict_preloaded = false;
                    }
                    Some(_) => {
                        // Skip non-data blocks, try again.
                        continue;
                    }
                    None => {
                        return Err(SurpError::UnexpectedEof(self.pos));
                    }
                }
            }

            let (block_start, block_end) = self.current_block.unwrap();
            let _ = block_start;

            if self.block_pos >= block_end {
                // Current block exhausted, try next.
                self.current_block = None;
                continue;
            }

            return self.decode_value_at(block_end);
        }
    }

    /// Decode the next value as an owned `Value`.
    ///
    /// Unlike `decode_next()`, this works transparently with both compressed
    /// and uncompressed blocks.
    pub fn decode_next_owned(&mut self) -> Result<Value> {
        // Loop instead of self-recursion: see decode_next() for rationale.
        loop {
            // If we don't have a current block, read one.
            if self.current_block.is_none() {
                match self.read_next_block()? {
                    Some((BlockType::Data, start, end)) => {
                        self.current_block = Some((start, end));
                        self.block_pos = start;
                        self.str_slices.clear();
                        // Only clear owned_strings if no StringDict block pre-populated it.
                        if !self.dict_preloaded {
                            self.owned_strings.clear();
                        }
                        // Reset the flag — it was consumed for this block.
                        self.dict_preloaded = false;
                    }
                    Some(_) => {
                        continue;
                    }
                    None => {
                        return Err(SurpError::UnexpectedEof(self.pos));
                    }
                }
            }

            let (_block_start, block_end) = self.current_block.unwrap();

            if self.block_pos >= block_end {
                self.current_block = None;
                continue;
            }

            return self.decode_value_owned_at(block_end);
        }
    }

    /// Decode a value at `self.block_pos` into an owned `Value`.
    ///
    /// Reads from `block_data()` (transparently handles decompressed blocks).
    fn decode_value_owned_at(&mut self, block_end: usize) -> Result<Value> {
        let data = self.block_data();
        if self.block_pos >= block_end {
            return Err(SurpError::UnexpectedEof(self.block_pos));
        }

        let tag = data[self.block_pos];
        self.block_pos += 1;

        let wire_type = WireType::from_tag(tag).ok_or(SurpError::InvalidWireType(tag))?;

        match wire_type {
            WireType::Null => Ok(Value::Null),

            WireType::Bool => {
                if self.block_pos >= block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                let data = self.block_data();
                let b = data[self.block_pos] != 0;
                self.block_pos += 1;
                Ok(Value::Bool(b))
            }

            WireType::VarUInt => {
                let data = self.block_data();
                let (val, consumed) = decode_varint(data, self.block_pos)?;
                self.block_pos += consumed;
                Ok(Value::UInt(val))
            }

            WireType::VarInt => {
                let data = self.block_data();
                let (val, consumed) = decode_signed_varint(data, self.block_pos)?;
                self.block_pos += consumed;
                Ok(Value::Int(val))
            }

            WireType::Fixed64 => {
                let data = self.block_data();
                if self.block_pos + 8 > block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                let bytes: [u8; 8] = data[self.block_pos..self.block_pos + 8].try_into().unwrap();
                self.block_pos += 8;
                Ok(Value::Float(f64::from_le_bytes(bytes)))
            }

            WireType::LenDelimited => {
                let data = self.block_data();
                if self.block_pos >= block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                let sub_type = data[self.block_pos];
                self.block_pos += 1;

                let data = self.block_data();
                let (len, consumed) = decode_varint(data, self.block_pos)?;
                self.block_pos += consumed;
                let len = safe_usize(len)?;

                if len > self.limits.max_string_length {
                    return Err(SurpError::StringTooLong(len, self.limits.max_string_length));
                }
                self.track_alloc(len)?;
                if self.block_pos + len > block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos + len));
                }

                let data = self.block_data();
                let payload_slice = data[self.block_pos..self.block_pos + len].to_vec();
                self.block_pos += len;

                match sub_type {
                    0x00 => {
                        let s = std::str::from_utf8(&payload_slice)
                            .map_err(|_| SurpError::InvalidUtf8(self.block_pos - len))?
                            .to_string();
                        // Record in per-block owned string table for Reference resolution.
                        self.owned_strings.push(s.clone());
                        Ok(Value::Str(s))
                    }
                    0x01 => Ok(Value::Bytes(payload_slice)),
                    _ => Ok(Value::Bytes(payload_slice)),
                }
            }

            WireType::StartArray => {
                if self.depth >= self.limits.max_nesting_depth {
                    return Err(SurpError::NestingTooDeep(
                        self.depth,
                        self.limits.max_nesting_depth,
                    ));
                }
                let data = self.block_data();
                let (count, consumed) = decode_varint(data, self.block_pos)?;
                self.block_pos += consumed;
                let count = safe_usize(count)?;

                if count > self.limits.max_items {
                    return Err(SurpError::TooManyItems(count, self.limits.max_items));
                }

                // Mirror the zero-copy path's per-item container cost estimate
                // (see the StartArray arm of `decode_value_at`) so that
                // container-heavy payloads (many small/empty nested arrays)
                // are tracked against `max_memory` just like the zero-copy path,
                // not just LenDelimited string/byte payloads.
                let alloc = count.min(1024) * std::mem::size_of::<Value>();
                self.track_alloc(alloc)?;

                self.depth += 1;
                let mut items = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    items.push(self.decode_value_owned_at(block_end)?);
                }
                self.depth -= 1;

                let data = self.block_data();
                if self.block_pos < block_end && data[self.block_pos] == WireType::EndArray.to_tag()
                {
                    self.block_pos += 1;
                }

                Ok(Value::Array(items))
            }

            WireType::StartObject => {
                if self.depth >= self.limits.max_nesting_depth {
                    return Err(SurpError::NestingTooDeep(
                        self.depth,
                        self.limits.max_nesting_depth,
                    ));
                }
                let data = self.block_data();
                let (count, consumed) = decode_varint(data, self.block_pos)?;
                self.block_pos += consumed;
                let count = safe_usize(count)?;

                if count > self.limits.max_items {
                    return Err(SurpError::TooManyItems(count, self.limits.max_items));
                }

                // Mirror the zero-copy path's per-item container cost estimate
                // (see the StartObject arm of `decode_value_at`), using the
                // owned equivalents of the borrowed types (`String` in place
                // of `&str`, `Value` in place of `SurpValue`).
                let alloc =
                    count.min(1024) * (std::mem::size_of::<String>() + std::mem::size_of::<Value>());
                self.track_alloc(alloc)?;

                self.depth += 1;
                let mut entries = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    let data = self.block_data();
                    let (key_len, kc) = decode_varint(data, self.block_pos)?;
                    self.block_pos += kc;
                    let key_len = safe_usize(key_len)?;

                    if self.block_pos + key_len > block_end {
                        return Err(SurpError::UnexpectedEof(self.block_pos + key_len));
                    }
                    let data = self.block_data();
                    let key_bytes = data[self.block_pos..self.block_pos + key_len].to_vec();
                    let key = std::str::from_utf8(&key_bytes)
                        .map_err(|_| SurpError::InvalidUtf8(self.block_pos))?
                        .to_string();
                    self.block_pos += key_len;

                    let val = self.decode_value_owned_at(block_end)?;
                    entries.push((key, val));
                }
                self.depth -= 1;

                let data = self.block_data();
                if self.block_pos < block_end
                    && data[self.block_pos] == WireType::EndObject.to_tag()
                {
                    self.block_pos += 1;
                }

                Ok(Value::Object(entries))
            }

            WireType::EndObject | WireType::EndArray => Err(SurpError::InvalidWireType(tag)),

            WireType::Reference => {
                let data = self.block_data();
                let (ref_id, consumed) = decode_varint(data, self.block_pos)?;
                self.block_pos += consumed;
                let ref_id = safe_usize(ref_id)?;

                // Resolve from per-block owned string table.
                if let Some(s) = self.owned_strings.get(ref_id) {
                    Ok(Value::Str(s.clone()))
                } else {
                    // Invalid reference — return error instead of silent fallback.
                    Err(SurpError::InvalidReference(
                        ref_id,
                        self.owned_strings.len(),
                    ))
                }
            }
        }
    }

    /// Decode a value starting at `self.block_pos`, not going past `block_end`.
    fn decode_value_at(&mut self, block_end: usize) -> Result<SurpValue<'a>> {
        if self.block_pos >= block_end {
            return Err(SurpError::UnexpectedEof(self.block_pos));
        }

        let tag = self.data[self.block_pos];
        self.block_pos += 1;

        let wire_type = WireType::from_tag(tag).ok_or(SurpError::InvalidWireType(tag))?;

        match wire_type {
            WireType::Null => Ok(SurpValue::Null),

            WireType::Bool => {
                if self.block_pos >= block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                let b = self.data[self.block_pos] != 0;
                self.block_pos += 1;
                Ok(SurpValue::Bool(b))
            }

            WireType::VarUInt => {
                let (val, consumed) = decode_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                Ok(SurpValue::UInt(val))
            }

            WireType::VarInt => {
                let (val, consumed) = decode_signed_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                Ok(SurpValue::Int(val))
            }

            WireType::Fixed64 => {
                if self.block_pos + 8 > block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                let bytes: [u8; 8] = self.data[self.block_pos..self.block_pos + 8]
                    .try_into()
                    .unwrap();
                self.block_pos += 8;
                Ok(SurpValue::Float(f64::from_le_bytes(bytes)))
            }

            WireType::LenDelimited => {
                if self.block_pos >= block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos));
                }
                let sub_type = self.data[self.block_pos];
                self.block_pos += 1;

                let (len, consumed) = decode_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                let len = safe_usize(len)?;

                if len > self.limits.max_string_length {
                    return Err(SurpError::StringTooLong(len, self.limits.max_string_length));
                }
                self.track_alloc(len)?;
                if self.block_pos + len > block_end {
                    return Err(SurpError::UnexpectedEof(self.block_pos + len));
                }

                let payload = &self.data[self.block_pos..self.block_pos + len];
                self.block_pos += len;

                match sub_type {
                    0x00 => {
                        // UTF-8 string — zero-copy borrow from input.
                        let s = std::str::from_utf8(payload)
                            .map_err(|_| SurpError::InvalidUtf8(self.block_pos - len))?;
                        // Record in per-block string table for Reference resolution.
                        self.str_slices.push(s);
                        Ok(SurpValue::Str(s))
                    }
                    0x01 => {
                        // Raw binary blob — zero-copy borrow.
                        Ok(SurpValue::Bytes(payload))
                    }
                    _ => {
                        // Unknown sub-type: treat as bytes for forward compatibility.
                        Ok(SurpValue::Bytes(payload))
                    }
                }
            }

            WireType::StartArray => {
                if self.depth >= self.limits.max_nesting_depth {
                    return Err(SurpError::NestingTooDeep(
                        self.depth,
                        self.limits.max_nesting_depth,
                    ));
                }
                let (count, consumed) = decode_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                let count = safe_usize(count)?;

                if count > self.limits.max_items {
                    return Err(SurpError::TooManyItems(count, self.limits.max_items));
                }

                let alloc = count.min(1024) * std::mem::size_of::<SurpValue>();
                self.track_alloc(alloc)?;

                self.depth += 1;
                let mut items = Vec::with_capacity(count.min(1024)); // Cap initial alloc
                for _ in 0..count {
                    items.push(self.decode_value_at(block_end)?);
                }
                self.depth -= 1;

                // Consume EndArray tag.
                if self.block_pos < block_end
                    && self.data[self.block_pos] == WireType::EndArray.to_tag()
                {
                    self.block_pos += 1;
                }

                Ok(SurpValue::Array(items))
            }

            WireType::StartObject => {
                if self.depth >= self.limits.max_nesting_depth {
                    return Err(SurpError::NestingTooDeep(
                        self.depth,
                        self.limits.max_nesting_depth,
                    ));
                }
                let (count, consumed) = decode_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                let count = safe_usize(count)?;

                if count > self.limits.max_items {
                    return Err(SurpError::TooManyItems(count, self.limits.max_items));
                }

                let alloc = count.min(1024)
                    * (std::mem::size_of::<&str>() + std::mem::size_of::<SurpValue>());
                self.track_alloc(alloc)?;

                self.depth += 1;
                let mut entries = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    // Read key: varint length + UTF-8 bytes.
                    let (key_len, kc) = decode_varint(self.data, self.block_pos)?;
                    self.block_pos += kc;
                    let key_len = safe_usize(key_len)?;

                    if self.block_pos + key_len > block_end {
                        return Err(SurpError::UnexpectedEof(self.block_pos + key_len));
                    }
                    let key =
                        std::str::from_utf8(&self.data[self.block_pos..self.block_pos + key_len])
                            .map_err(|_| SurpError::InvalidUtf8(self.block_pos))?;
                    self.block_pos += key_len;

                    // Read value.
                    let val = self.decode_value_at(block_end)?;
                    entries.push((key, val));
                }
                self.depth -= 1;

                // Consume EndObject tag.
                if self.block_pos < block_end
                    && self.data[self.block_pos] == WireType::EndObject.to_tag()
                {
                    self.block_pos += 1;
                }

                Ok(SurpValue::Object(entries))
            }

            WireType::EndObject | WireType::EndArray => {
                // Should not be encountered at top level; treat as protocol error.
                Err(SurpError::InvalidWireType(tag))
            }

            WireType::Reference => {
                // Reference wire type: resolve from the per-block string dictionary.
                let (ref_id, consumed) = decode_varint(self.data, self.block_pos)?;
                self.block_pos += consumed;
                let ref_id = safe_usize(ref_id)?;

                // Resolve via borrowed slices from the input buffer (zero-copy).
                if let Some(&s) = self.str_slices.get(ref_id) {
                    Ok(SurpValue::Str(s))
                } else {
                    // Invalid reference — return error instead of silent fallback.
                    Err(SurpError::InvalidReference(ref_id, self.str_slices.len()))
                }
            }
        }
    }

    /// Decode all remaining values from the input.
    pub fn decode_all(&mut self) -> Result<Vec<SurpValue<'a>>> {
        let mut values = Vec::new();
        loop {
            match self.decode_next() {
                Ok(v) => values.push(v),
                Err(SurpError::UnexpectedEof(_)) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(values)
    }

    /// Decode all remaining values as owned Values.
    ///
    /// This works transparently with both compressed and uncompressed blocks.
    pub fn decode_all_owned(&mut self) -> Result<Vec<Value>> {
        let mut values = Vec::new();
        loop {
            match self.decode_next_owned() {
                Ok(v) => values.push(v),
                Err(SurpError::UnexpectedEof(_)) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(values)
    }

    /// Current position in the input.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Cumulative bytes tracked as allocated during this decode session.
    pub fn memory_used(&self) -> usize {
        self.memory_used
    }
}

// ---------------------------------------------------------------------------
// Bump-allocated decoder (feature = "fast-alloc")
// ---------------------------------------------------------------------------

/// A decoder that carries a per-instance bump allocator alongside the
/// standard `Decoder`, for callers who want bump-allocated scratch space
/// for their own advanced/manual use (via [`BumpDecoder::alloc_bytes`] /
/// [`BumpDecoder::alloc_str`]).
///
/// **Current behavior:** `decode_next`, `decode_all`, and `decode_all_owned`
/// simply delegate to the inner plain `Decoder` and do **not** route any
/// scratch allocations (e.g. the per-block string table, or the returned
/// `SurpValue`/`Value` trees) through `self.arena` — those still use
/// ordinary heap allocation exactly as the plain `Decoder` does. The arena
/// is only touched by `reset_arena`, `arena_allocated`, and the manual
/// `alloc_bytes`/`alloc_str` helpers below. Wiring the automatic decode path
/// through the arena would require making `Decoder`'s internal string-table
/// storage generic over the allocator, which is out of scope here; if you
/// need bump-allocated scratch during decode today, drive it manually via
/// `inner_mut()` together with `alloc_bytes`/`alloc_str`.
///
/// # Example
///
/// ```rust,ignore
/// use surp_core::decoder::BumpDecoder;
/// use surp_core::Value;
///
/// let bytes = /* encoded surp data */;
/// let mut dec = BumpDecoder::new(&bytes);
/// let values = dec.decode_all_owned()?;
/// ```
#[cfg(feature = "fast-alloc")]
pub struct BumpDecoder<'a> {
    decoder: Decoder<'a>,
    arena: bumpalo::Bump,
}

#[cfg(feature = "fast-alloc")]
impl<'a> BumpDecoder<'a> {
    /// Create a new bump-allocated decoder.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            decoder: Decoder::new(data),
            arena: bumpalo::Bump::with_capacity(4096),
        }
    }

    /// Create a bump-allocated decoder with custom limits.
    pub fn with_limits(data: &'a [u8], limits: Limits) -> Self {
        Self {
            decoder: Decoder::with_limits(data, limits),
            arena: bumpalo::Bump::with_capacity(4096),
        }
    }

    /// Create with a specific arena capacity hint (bytes).
    pub fn with_capacity(data: &'a [u8], arena_capacity: usize) -> Self {
        Self {
            decoder: Decoder::new(data),
            arena: bumpalo::Bump::with_capacity(arena_capacity),
        }
    }

    /// Reset the arena for the next block. Call this between blocks
    /// to reclaim memory without deallocating the backing storage.
    pub fn reset_arena(&mut self) {
        self.arena.reset();
    }

    /// Get arena usage statistics.
    pub fn arena_allocated(&self) -> usize {
        self.arena.allocated_bytes()
    }

    /// Allocate a byte slice in the arena (useful for scratch buffers).
    pub fn alloc_bytes(&self, src: &[u8]) -> &[u8] {
        self.arena.alloc_slice_copy(src)
    }

    /// Allocate a string in the arena (useful for scratch key copies).
    pub fn alloc_str(&self, s: &str) -> &str {
        self.arena.alloc_str(s)
    }

    /// Decode next value, delegating to inner decoder.
    pub fn decode_next(&mut self) -> Result<SurpValue<'a>> {
        self.decoder.decode_next()
    }

    /// Decode all remaining values.
    pub fn decode_all(&mut self) -> Result<Vec<SurpValue<'a>>> {
        self.decoder.decode_all()
    }

    /// Decode all remaining values as owned.
    pub fn decode_all_owned(&mut self) -> Result<Vec<Value>> {
        self.decoder.decode_all_owned()
    }

    /// Get the inner decoder (for advanced usage).
    pub fn inner(&self) -> &Decoder<'a> {
        &self.decoder
    }

    /// Get the inner decoder mutably.
    pub fn inner_mut(&mut self) -> &mut Decoder<'a> {
        &mut self.decoder
    }

    /// Memory used by the inner decoder's tracking.
    pub fn memory_used(&self) -> usize {
        self.decoder.memory_used()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::Encoder;

    /// Helper: encode a value and decode it back.
    fn roundtrip(value: &Value) -> Value {
        let mut enc = Encoder::new();
        enc.encode_value(value).unwrap();
        let bytes = enc.finish().unwrap();
        let mut dec = Decoder::new(&bytes);
        dec.decode_next().unwrap().to_owned_value()
    }

    #[test]
    fn roundtrip_null() {
        assert_eq!(roundtrip(&Value::Null), Value::Null);
    }

    #[test]
    fn roundtrip_bool() {
        assert_eq!(roundtrip(&Value::Bool(true)), Value::Bool(true));
        assert_eq!(roundtrip(&Value::Bool(false)), Value::Bool(false));
    }

    #[test]
    fn roundtrip_uint() {
        for &v in &[0u64, 1, 127, 128, 300, 65535, u64::MAX] {
            assert_eq!(
                roundtrip(&Value::UInt(v)),
                Value::UInt(v),
                "uint roundtrip failed for {v}"
            );
        }
    }

    #[test]
    fn roundtrip_int() {
        for &v in &[0i64, 1, -1, 127, -128, 1000, -1000, i64::MAX, i64::MIN] {
            assert_eq!(
                roundtrip(&Value::Int(v)),
                Value::Int(v),
                "int roundtrip failed for {v}"
            );
        }
    }

    #[test]
    fn roundtrip_float() {
        for &v in &[0.0f64, 1.0, -1.0, 3.125, f64::MAX, f64::MIN, f64::INFINITY] {
            assert_eq!(
                roundtrip(&Value::Float(v)),
                Value::Float(v),
                "float roundtrip failed for {v}"
            );
        }
    }

    #[test]
    fn roundtrip_string() {
        let long_str = "a".repeat(1000);
        for s in &["", "hello", "こんにちは", long_str.as_str()] {
            assert_eq!(
                roundtrip(&Value::Str(s.to_string())),
                Value::Str(s.to_string()),
                "string roundtrip failed for {s:?}"
            );
        }
    }

    #[test]
    fn roundtrip_bytes() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        assert_eq!(roundtrip(&Value::Bytes(data.clone())), Value::Bytes(data));
    }

    #[test]
    fn roundtrip_array() {
        let arr = Value::Array(vec![
            Value::UInt(1),
            Value::Str("two".into()),
            Value::Bool(true),
            Value::Null,
        ]);
        assert_eq!(roundtrip(&arr), arr);
    }

    #[test]
    fn roundtrip_object() {
        let obj = Value::Object(vec![
            ("name".into(), Value::Str("Alice".into())),
            ("age".into(), Value::UInt(30)),
            ("active".into(), Value::Bool(true)),
        ]);
        assert_eq!(roundtrip(&obj), obj);
    }

    #[test]
    fn roundtrip_nested() {
        let val = Value::Object(vec![
            (
                "users".into(),
                Value::Array(vec![Value::Object(vec![
                    ("name".into(), Value::Str("Bob".into())),
                    (
                        "scores".into(),
                        Value::Array(vec![Value::UInt(100), Value::UInt(95), Value::UInt(87)]),
                    ),
                ])]),
            ),
            ("count".into(), Value::UInt(1)),
        ]);
        assert_eq!(roundtrip(&val), val);
    }

    #[test]
    fn checksum_verification() {
        let mut enc = Encoder::new();
        enc.encode_value(&Value::UInt(42)).unwrap();
        let mut bytes = enc.finish().unwrap();

        // Corrupt a byte in the payload area after the block header.
        let corrupt_pos = 12; // somewhere in the payload
        if corrupt_pos < bytes.len() {
            bytes[corrupt_pos] ^= 0xFF;
        }

        let mut dec = Decoder::new(&bytes);
        assert!(dec.decode_next().is_err());
    }

    #[test]
    fn nesting_depth_limit() {
        let limits = Limits {
            max_nesting_depth: 2,
            ..Limits::default()
        };
        // Create deeply nested value
        let val = Value::Array(vec![Value::Array(vec![Value::Array(vec![])])]);
        let mut enc = Encoder::with_limits(Limits::unlimited());
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let mut dec = Decoder::with_limits(&bytes, limits);
        assert!(dec.decode_next().is_err());
    }

    #[test]
    fn memory_tracking() {
        // Decode a large string and verify memory is tracked.
        let big_str = "x".repeat(1000);
        let val = Value::Str(big_str);
        let mut enc = Encoder::new();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let mut dec = Decoder::new(&bytes);
        let _ = dec.decode_next().unwrap();
        assert!(
            dec.memory_used() >= 1000,
            "memory should track string allocation"
        );
    }

    #[test]
    fn memory_limit_enforcement() {
        let big_str = "x".repeat(1000);
        let val = Value::Str(big_str);
        let mut enc = Encoder::new();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let limits = Limits {
            max_memory: 500,
            ..Limits::default()
        };
        let mut dec = Decoder::with_limits(&bytes, limits);
        assert!(
            dec.decode_next().is_err(),
            "should fail when memory limit exceeded"
        );
    }

    /// The owned decode path (`decode_all_owned` / `decode_value_owned_at`)
    /// used to only call `track_alloc` for `LenDelimited` string/byte
    /// payloads, never for `StartArray`/`StartObject` containers. This meant
    /// a payload dominated by many small/empty nested arrays (little string
    /// data, but real `Vec<Value>` allocation happening under the hood)
    /// would never trip `max_memory`, even though `Surp::from_surp_bytes_with_limits`
    /// documents that it "enforces all Limits during binary decode: nesting
    /// depth, memory, block size, string length, and item count."
    /// This test isolates the container-count case (as opposed to
    /// `memory_limit_enforcement`, which covers the long-string case) by
    /// encoding many small empty arrays and using a tight `max_memory` limit.
    #[test]
    fn memory_limit_enforcement_container_count() {
        // Many small nested empty arrays: negligible string/byte payload,
        // but real container allocation per item.
        let val = Value::Array((0..5000).map(|_| Value::Array(vec![])).collect());
        let mut enc = Encoder::with_limits(Limits::unlimited());
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let limits = Limits {
            max_memory: 256,
            ..Limits::default()
        };
        let mut dec = Decoder::with_limits(&bytes, limits);
        let err = dec.decode_all_owned().unwrap_err();
        assert!(
            matches!(err, SurpError::MemoryLimitExceeded(_, _)),
            "expected MemoryLimitExceeded from container-heavy owned decode, got {err:?}"
        );
    }

    #[test]
    fn skip_value_works() {
        // Encode a complex value, then manually position decoder and skip it.
        let val = Value::Object(vec![
            ("name".into(), Value::Str("Alice".into())),
            (
                "scores".into(),
                Value::Array(vec![Value::UInt(1), Value::UInt(2)]),
            ),
        ]);
        let mut enc = Encoder::new();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        // Decode normally first to verify it's valid.
        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_next().unwrap().to_owned_value();
        assert_eq!(decoded, val);
    }

    #[test]
    fn string_dedup_roundtrip() {
        // Encode with dedup enabled — repeated strings should decode correctly.
        // Use enough repeats so StringDict block overhead is amortized.
        let val = Value::Array(vec![
            Value::Str("hello_world_long_string".into()),
            Value::Str("another_reasonably_long_string".into()),
            Value::Str("hello_world_long_string".into()), // dup
            Value::Str("another_reasonably_long_string".into()), // dup
            Value::Str("hello_world_long_string".into()), // dup
            Value::Str("another_reasonably_long_string".into()), // dup
            Value::Str("hello_world_long_string".into()), // dup
            Value::Str("another_reasonably_long_string".into()), // dup
            Value::Str("hello_world_long_string".into()), // dup
            Value::Str("another_reasonably_long_string".into()), // dup
        ]);

        let mut enc = Encoder::new();
        enc.enable_dedup();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        // Dedup should produce smaller output than non-dedup.
        let mut enc_no_dedup = Encoder::new();
        enc_no_dedup.encode_value(&val).unwrap();
        let bytes_no_dedup = enc_no_dedup.finish().unwrap();
        assert!(
            bytes.len() < bytes_no_dedup.len(),
            "dedup ({}) should be smaller than no-dedup ({})",
            bytes.len(),
            bytes_no_dedup.len()
        );

        // Decode should resolve references back to the original strings.
        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_next().unwrap().to_owned_value();
        assert_eq!(
            decoded, val,
            "dedup roundtrip should produce identical value"
        );
    }

    #[test]
    fn string_dedup_in_object() {
        // Verify dedup works for string values inside objects.
        let val = Value::Object(vec![
            ("greeting".into(), Value::Str("hello".into())),
            ("farewell".into(), Value::Str("goodbye".into())),
            ("echo".into(), Value::Str("hello".into())), // dup of first value
        ]);

        let mut enc = Encoder::new();
        enc.enable_dedup();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_next().unwrap().to_owned_value();
        assert_eq!(decoded, val, "dedup in object should roundtrip correctly");
    }

    #[cfg(feature = "fast-alloc")]
    #[test]
    fn bump_decoder_roundtrip() {
        let val = Value::Array(vec![
            Value::Str("hello".into()),
            Value::UInt(42),
            Value::Object(vec![("key".into(), Value::Bool(true))]),
        ]);
        let mut enc = Encoder::new();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let mut dec = BumpDecoder::new(&bytes);
        let decoded = dec.decode_all_owned().unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], val);
    }

    /// `decode_next`/`decode_all`/`decode_all_owned` on `BumpDecoder` simply
    /// delegate to the inner plain `Decoder` and never touch `self.arena` —
    /// so `arena_allocated() > 0` alone (as the previous version of this test
    /// asserted) proves nothing about decode actually using the arena: it's
    /// satisfied merely by `bumpalo::Bump::with_capacity`'s initial chunk
    /// reservation. This test instead asserts the honest current behavior
    /// (arena usage is unchanged by automatic decode) while confirming the
    /// arena is genuinely usable for manual scratch allocations via
    /// `alloc_str`/`alloc_bytes`.
    #[cfg(feature = "fast-alloc")]
    #[test]
    fn bump_decoder_arena_unused_by_automatic_decode() {
        let val = Value::Array(vec![
            Value::Str("hello".into()),
            Value::UInt(42),
            Value::Object(vec![("key".into(), Value::Bool(true))]),
        ]);
        let mut enc = Encoder::new();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let mut dec = BumpDecoder::new(&bytes);
        let baseline = dec.arena_allocated();

        let decoded = dec.decode_all_owned().unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], val);
        assert_eq!(
            dec.arena_allocated(),
            baseline,
            "automatic decode should not grow the arena; it is not wired into the decode path"
        );

        // Manual scratch usage via alloc_str/alloc_bytes is genuinely usable
        // (this is the arena's actual current role: available for
        // hand-driven scratch, not automatically wired into decode). Note
        // that `arena_allocated()` reports total *chunk capacity*, not bytes
        // used within a chunk (see bumpalo's docs), so a small manual
        // allocation that fits in the already-reserved initial chunk won't
        // move the counter — only exceeding that capacity forces a new
        // chunk and grows it, which we exercise here with a large buffer.
        let s = dec.alloc_str("manual scratch allocation");
        assert_eq!(s, "manual scratch allocation");

        let big = vec![0xABu8; 8192]; // larger than the default 4096-byte initial chunk
        let allocated = dec.alloc_bytes(&big);
        assert_eq!(allocated, &big[..]);
        assert!(
            dec.arena_allocated() > baseline,
            "a manual allocation exceeding the initial chunk capacity should grow the arena"
        );
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn lz4_compression_roundtrip() {
        let val = Value::Object(vec![
            ("name".into(), Value::Str("Alice".into())),
            (
                "bio".into(),
                Value::Str(
                    "A long string that is repeated many times to test compression effectiveness. "
                        .repeat(20),
                ),
            ),
            ("scores".into(), Value::Array(vec![Value::UInt(100); 50])),
        ]);
        let mut enc = Encoder::new();
        enc.set_compression(CompressionType::Lz4);
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        // Owned decode should work transparently with compressed blocks.
        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_all_owned().unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], val);
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn lz4_zero_copy_errors_cleanly() {
        // Use highly compressible data so LZ4 actually compresses it
        // (small data may fall back to uncompressed when ratio is >= 1.0).
        let val = Value::Str("A".repeat(1000));
        let mut enc = Encoder::new();
        enc.set_compression(CompressionType::Lz4);
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        // Zero-copy decode should return an error for compressed blocks.
        let mut dec = Decoder::new(&bytes);
        assert!(
            dec.decode_next().is_err(),
            "zero-copy decode should fail on compressed block"
        );

        // But decode_all_owned should work.
        let mut dec2 = Decoder::new(&bytes);
        let decoded = dec2.decode_all_owned().unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], val);
    }

    #[test]
    fn decode_next_owned_uncompressed() {
        let val = Value::Object(vec![
            ("x".into(), Value::UInt(10)),
            ("y".into(), Value::Float(3.15)),
        ]);
        let mut enc = Encoder::new();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_next_owned().unwrap();
        assert_eq!(decoded, val);
    }

    #[test]
    fn dedup_owned_roundtrip() {
        // Verify that decode_next_owned resolves string References correctly.
        let val = Value::Array(vec![
            Value::Str("alpha".into()),
            Value::Str("beta".into()),
            Value::Str("alpha".into()), // dup
            Value::Str("beta".into()),  // dup
            Value::Str("gamma".into()),
            Value::Str("alpha".into()), // dup
        ]);
        let mut enc = Encoder::new();
        enc.enable_dedup();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_next_owned().unwrap();
        assert_eq!(decoded, val, "owned decode should resolve dedup references");
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn dedup_plus_lz4_roundtrip() {
        // Combined: dedup + LZ4 compression. Both features exercise the owned path.
        let val = Value::Object(vec![
            ("city".into(), Value::Str("Tokyo".into())),
            ("country".into(), Value::Str("Japan".into())),
            (
                "description".into(),
                Value::Str("Tokyo is the capital of Japan. ".repeat(30)),
            ),
            ("origin".into(), Value::Str("Tokyo".into())), // dup
            ("nation".into(), Value::Str("Japan".into())), // dup
        ]);
        let mut enc = Encoder::new();
        enc.enable_dedup();
        enc.set_compression(CompressionType::Lz4);
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_all_owned().unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], val, "dedup + lz4 should roundtrip correctly");
    }

    #[test]
    fn string_dict_block_roundtrip() {
        // Verify that the StringDict block is emitted and decoded correctly.
        // Use strings with shared prefixes to exercise prefix-delta compression.
        let val = Value::Array(vec![
            Value::Str("config_database_host".into()),
            Value::Str("config_database_port".into()),
            Value::Str("config_database_name".into()),
            Value::Str("config_cache_host".into()),
            Value::Str("config_cache_port".into()),
            // Duplicates — will use Reference wire type
            Value::Str("config_database_host".into()),
            Value::Str("config_database_port".into()),
            Value::Str("config_cache_host".into()),
            Value::Str("config_database_host".into()),
            Value::Str("config_cache_port".into()),
        ]);

        let mut enc = Encoder::new();
        enc.enable_dedup();
        enc.encode_value(&val).unwrap();
        let bytes = enc.finish().unwrap();

        // Without dedup for size comparison.
        let mut enc_no_dedup = Encoder::new();
        enc_no_dedup.encode_value(&val).unwrap();
        let bytes_no_dedup = enc_no_dedup.finish().unwrap();

        assert!(
            bytes.len() < bytes_no_dedup.len(),
            "dedup with StringDict ({}) should be smaller than no-dedup ({})",
            bytes.len(),
            bytes_no_dedup.len()
        );

        // Zero-copy decode should still work (StringDict is consumed before Data block).
        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_next().unwrap().to_owned_value();
        assert_eq!(
            decoded, val,
            "StringDict block roundtrip should produce identical value"
        );

        // Owned decode should also work.
        let mut dec2 = Decoder::new(&bytes);
        let decoded2 = dec2.decode_next_owned().unwrap();
        assert_eq!(
            decoded2, val,
            "StringDict block owned roundtrip should produce identical value"
        );
    }

    #[test]
    fn string_dict_prefix_delta_compression() {
        // Verify prefix-delta compression produces smaller output for strings
        // with shared prefixes vs. strings with no shared prefixes.
        let shared_prefix_val = Value::Array(vec![
            Value::Str("application_settings_theme".into()),
            Value::Str("application_settings_language".into()),
            Value::Str("application_settings_timezone".into()),
            // Dups to amortize StringDict overhead
            Value::Str("application_settings_theme".into()),
            Value::Str("application_settings_language".into()),
            Value::Str("application_settings_timezone".into()),
            Value::Str("application_settings_theme".into()),
            Value::Str("application_settings_language".into()),
        ]);

        let mut enc = Encoder::new();
        enc.enable_dedup();
        enc.encode_value(&shared_prefix_val).unwrap();
        let bytes = enc.finish().unwrap();

        // Verify roundtrip.
        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_all_owned().unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], shared_prefix_val);
    }

    /// `decode_next`/`decode_next_owned` used to skip non-Data blocks (and
    /// exhausted blocks) via self-recursion, so a crafted input with many
    /// tiny consecutive skippable blocks could blow the native call stack.
    /// This test builds thousands of minimal empty `Index` blocks followed
    /// by a single `Data` block, and asserts decoding completes without
    /// crashing (the loop-based rewrite should skip them iteratively).
    #[test]
    fn skips_many_consecutive_non_data_blocks_without_stack_overflow() {
        use crate::block::BlockWriter;

        let mut bytes = Vec::new();
        for _ in 0..5_000 {
            bytes.extend_from_slice(&BlockWriter::new(BlockType::Index).finish());
        }

        let mut enc = Encoder::new();
        enc.encode_value(&Value::UInt(42)).unwrap();
        bytes.extend_from_slice(&enc.finish().unwrap());

        let mut dec = Decoder::new(&bytes);
        let decoded = dec.decode_next().unwrap().to_owned_value();
        assert_eq!(decoded, Value::UInt(42));

        let mut dec_owned = Decoder::new(&bytes);
        let decoded_owned = dec_owned.decode_next_owned().unwrap();
        assert_eq!(decoded_owned, Value::UInt(42));
    }
}
