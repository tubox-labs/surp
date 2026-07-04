//! Block-level framing for the Surp binary format.
//!
//! Blocks are the fundamental container for data in a Surp file.
//! Each block has:
//! - A type byte identifying the block kind (data, index, schema, etc.)
//! - A varint-encoded length
//! - A compression type byte
//! - An 8-byte XXH64 checksum of the (uncompressed) payload
//! - The payload bytes
//!
//! This module provides `BlockWriter` for building blocks incrementally
//! and `BlockReader` for reading them from a byte slice.

use crate::checksum::compute_xxh64;
use crate::error::{Result, SurpError};
use crate::varint::{decode_varint, encode_varint_vec};
use crate::wire::{BlockType, CompressionType};

/// Builder for a single block's payload, with framing support.
pub struct BlockWriter {
    block_type: BlockType,
    compression: CompressionType,
    payload: Vec<u8>,
}

impl BlockWriter {
    /// Create a new block writer of the given type.
    pub fn new(block_type: BlockType) -> Self {
        Self {
            block_type,
            compression: CompressionType::None,
            payload: Vec::with_capacity(4096),
        }
    }

    /// Set the compression type for this block.
    pub fn set_compression(&mut self, comp: CompressionType) {
        self.compression = comp;
    }

    /// Write raw bytes into the block payload.
    pub fn write(&mut self, data: &[u8]) {
        self.payload.extend_from_slice(data);
    }

    /// Get a mutable reference to the payload buffer.
    pub fn payload_mut(&mut self) -> &mut Vec<u8> {
        &mut self.payload
    }

    /// Get the current payload size.
    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }

    /// Finalize and serialize this block into a byte vector.
    ///
    /// Layout: `block_type(1)` | `block_len(varint)` | `comp_type(1)` | `checksum(8)` | `payload`
    pub fn finish(self) -> Vec<u8> {
        let checksum = compute_xxh64(&self.payload);
        let mut out = Vec::with_capacity(1 + 10 + 1 + 8 + self.payload.len());

        out.push(self.block_type as u8);
        encode_varint_vec(self.payload.len() as u64, &mut out);
        out.push(self.compression as u8);
        out.extend_from_slice(&checksum.to_le_bytes());
        out.extend_from_slice(&self.payload);

        out
    }
}

/// A parsed block read from binary data.
#[derive(Debug)]
pub struct BlockReader<'a> {
    /// The block type.
    pub block_type: BlockType,
    /// The compression type.
    pub compression: CompressionType,
    /// The expected checksum.
    pub checksum: u64,
    /// The payload bytes (borrowed from input).
    pub payload: &'a [u8],
}

impl<'a> BlockReader<'a> {
    /// Parse a block from `data` starting at `offset`.
    /// Returns `(BlockReader, bytes_consumed)`.
    ///
    /// This variant applies no upper bound on the declared `block_len` beyond
    /// what fits in `data` — it is kept for backward compatibility. Direct
    /// consumers of `BlockReader` that read untrusted input should prefer
    /// [`Self::parse_with_limit`], which mirrors the `max_block_size` guard
    /// that [`crate::decoder::Decoder`] already enforces.
    pub fn parse(data: &'a [u8], offset: usize) -> Result<(Self, usize)> {
        Self::parse_inner(data, offset, None)
    }

    /// Parse a block from `data` starting at `offset`, rejecting a declared
    /// `block_len` that exceeds `max_block_size`.
    ///
    /// Unlike [`Self::parse`], this bounds the payload length *before* it is
    /// used to slice `data`, giving direct consumers of `BlockReader` (e.g.
    /// via the re-exported `surp_core::block::BlockReader`) the same
    /// size-guard protection that [`crate::decoder::Decoder`] provides via
    /// [`crate::limits::Limits::max_block_size`].
    pub fn parse_with_limit(
        data: &'a [u8],
        offset: usize,
        max_block_size: usize,
    ) -> Result<(Self, usize)> {
        Self::parse_inner(data, offset, Some(max_block_size))
    }

    fn parse_inner(
        data: &'a [u8],
        offset: usize,
        max_block_size: Option<usize>,
    ) -> Result<(Self, usize)> {
        let mut pos = offset;

        if pos >= data.len() {
            return Err(SurpError::UnexpectedEof(pos));
        }

        let block_type_byte = data[pos];
        pos += 1;
        let block_type = BlockType::from_byte(block_type_byte)
            .ok_or(SurpError::InvalidBlockType(block_type_byte))?;

        let (block_len, varint_bytes) = decode_varint(data, pos)?;
        pos += varint_bytes;
        let block_len = block_len as usize;

        if let Some(max_block_size) = max_block_size {
            if block_len > max_block_size {
                return Err(SurpError::BlockTooLarge(block_len, max_block_size));
            }
        }

        if pos >= data.len() {
            return Err(SurpError::UnexpectedEof(pos));
        }
        let comp_byte = data[pos];
        pos += 1;
        let compression = CompressionType::from_byte(comp_byte)
            .ok_or(SurpError::UnknownCompression(comp_byte))?;

        if pos + 8 > data.len() {
            return Err(SurpError::UnexpectedEof(pos));
        }
        let checksum = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        if pos
            .checked_add(block_len)
            .is_none_or(|end| end > data.len())
        {
            return Err(SurpError::UnexpectedEof(pos));
        }
        let payload = &data[pos..pos + block_len];
        pos += block_len;

        Ok((
            Self {
                block_type,
                compression,
                checksum,
                payload,
            },
            pos - offset,
        ))
    }

    /// Verify the block's checksum matches its payload.
    pub fn verify_checksum(&self) -> bool {
        compute_xxh64(self.payload) == self.checksum
    }
}

// ---------------------------------------------------------------------------
// Shared block-walking / inspection algorithm
// ---------------------------------------------------------------------------

/// Per-block summary computed by [`inspect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRecord {
    /// Zero-based position of this block within the file.
    pub index: usize,
    /// Byte offset of this block's header within the file.
    pub offset: usize,
    /// The block's declared type.
    pub block_type: BlockType,
    /// The block's declared compression type.
    pub compression: CompressionType,
    /// Length of the (possibly-compressed) on-wire payload, in bytes.
    pub payload_len: usize,
    /// Whether the block's checksum could be verified, and if so, whether it
    /// matched. `None` means the block is compressed, so verifying its
    /// checksum would require decompressing it first (not done here).
    pub payload_checksum_valid: Option<bool>,
}

/// Trailer-block summary computed by [`inspect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrailerRecord {
    /// Whether the trailer block's own payload checksum matches.
    pub payload_checksum_valid: bool,
    /// Whether the trailer's embedded whole-file checksum matches the
    /// checksum of every byte preceding the trailer block.
    pub file_checksum_valid: bool,
}

/// Result of walking every block in a Surp binary file via [`inspect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockReport {
    /// Total size of the inspected data, in bytes.
    pub file_size: usize,
    /// Per-block summaries, in file order.
    pub blocks: Vec<BlockRecord>,
    /// Trailer summary, if a trailer block was found.
    pub trailer: Option<TrailerRecord>,
}

/// Walk every block in `data`, verifying per-block checksums where possible
/// (uncompressed blocks, and the trailer block) and validating the file
/// trailer's whole-file checksum.
///
/// This is the single shared implementation of the block-walking/
/// verification algorithm used by both `surp-cli`'s `inspect`/`validate`
/// commands and `surp-mcp`'s `surp_v1_inspect`/`surp_v1_validate` tools —
/// previously each crate duplicated this loop independently.
pub fn inspect(data: &[u8]) -> Result<BlockReport> {
    let mut blocks = Vec::new();
    let mut trailer = None;

    let mut offset = 0usize;
    let mut index = 0usize;
    while offset < data.len() {
        let block_offset = offset;
        let (block, consumed) = BlockReader::parse(data, offset).map_err(|err| {
            SurpError::InvalidData(format!(
                "failed to parse block #{index} at offset {offset}: {err}"
            ))
        })?;
        offset += consumed;

        let payload_checksum_valid = if block.compression == CompressionType::None
            || block.block_type == BlockType::Trailer
        {
            Some(block.verify_checksum())
        } else {
            None
        };

        if block.block_type == BlockType::Trailer {
            let file_checksum_valid = if block.payload.len() == 8 {
                let mut expected_bytes = [0u8; 8];
                expected_bytes.copy_from_slice(block.payload);
                let expected = u64::from_le_bytes(expected_bytes);
                let actual = compute_xxh64(&data[..block_offset]);
                expected == actual
            } else {
                false
            };

            trailer = Some(TrailerRecord {
                payload_checksum_valid: block.verify_checksum(),
                file_checksum_valid,
            });
        }

        blocks.push(BlockRecord {
            index,
            offset: block_offset,
            block_type: block.block_type,
            compression: block.compression,
            payload_len: block.payload.len(),
            payload_checksum_valid,
        });
        index += 1;
    }

    Ok(BlockReport {
        file_size: data.len(),
        blocks,
        trailer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_roundtrip() {
        let mut writer = BlockWriter::new(BlockType::Data);
        writer.write(b"hello world");
        let bytes = writer.finish();

        let (reader, consumed) = BlockReader::parse(&bytes, 0).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(reader.block_type, BlockType::Data);
        assert_eq!(reader.compression, CompressionType::None);
        assert_eq!(reader.payload, b"hello world");
        assert!(reader.verify_checksum());
    }

    #[test]
    fn block_checksum_failure() {
        let mut writer = BlockWriter::new(BlockType::Data);
        writer.write(b"test data");
        let mut bytes = writer.finish();

        // Corrupt last byte of payload.
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;

        let (reader, _) = BlockReader::parse(&bytes, 0).unwrap();
        assert!(!reader.verify_checksum());
    }

    #[test]
    fn parse_with_limit_rejects_oversized_declared_block_len() {
        // Craft a block header that declares a block_len far larger than the
        // cap, but with a real payload that's actually small enough to fit
        // in `data` — the point is the guard rejects the *declared* length
        // before it's ever used for slicing/allocation.
        let mut writer = BlockWriter::new(BlockType::Data);
        writer.write(b"hello world");
        let bytes = writer.finish();

        // Unguarded parse succeeds.
        let (reader, _) = BlockReader::parse(&bytes, 0).unwrap();
        assert_eq!(reader.payload, b"hello world");

        // Guarded parse with a cap smaller than the payload is rejected cleanly.
        let err = BlockReader::parse_with_limit(&bytes, 0, 4).unwrap_err();
        assert!(matches!(err, SurpError::BlockTooLarge(_, 4)));

        // Guarded parse with a sufficient cap still succeeds.
        let (reader, consumed) = BlockReader::parse_with_limit(&bytes, 0, 4096).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(reader.payload, b"hello world");
    }

    #[test]
    fn inspect_walks_blocks_and_validates_trailer() {
        use crate::encoder::Encoder;
        use crate::value::Value;

        let mut encoder = Encoder::new();
        encoder
            .encode_value(&Value::Object(vec![("hello".into(), Value::UInt(42))]))
            .unwrap();
        let bytes = encoder.finish().unwrap();

        let report = inspect(&bytes).unwrap();
        assert_eq!(report.file_size, bytes.len());
        assert!(!report.blocks.is_empty());
        assert_eq!(report.blocks[0].block_type, BlockType::Data);
        assert_eq!(report.blocks[0].compression, CompressionType::None);
        assert_eq!(report.blocks[0].payload_checksum_valid, Some(true));

        let trailer = report.trailer.expect("trailer must exist");
        assert!(trailer.payload_checksum_valid);
        assert!(trailer.file_checksum_valid);
    }

    #[test]
    fn inspect_reports_corrupt_block_cleanly() {
        // Truncate the data so a block header can't be fully parsed.
        let mut writer = BlockWriter::new(BlockType::Data);
        writer.write(b"hello world");
        let mut bytes = writer.finish();
        bytes.truncate(3);

        let err = inspect(&bytes).unwrap_err();
        assert!(err.to_string().contains("failed to parse block #0"));
    }

    #[test]
    fn block_types() {
        for bt in [
            BlockType::Data,
            BlockType::Index,
            BlockType::Schema,
            BlockType::StringDict,
        ] {
            let writer = BlockWriter::new(bt);
            let bytes = writer.finish();
            let (reader, _) = BlockReader::parse(&bytes, 0).unwrap();
            assert_eq!(reader.block_type, bt);
        }
    }
}
