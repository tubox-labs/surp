//! # surp-io
//!
//! Async IO adapters for Surp, including:
//! - Framed stream reader/writer for Tokio
//! - Memory-mapped file reader (feature `mmap`)
//! - Streaming block reader
//! - Bytes-based shared buffer API
//!
//! ## Feature flags
//! - `mmap` — enables `MmapReader` for zero-copy file access.
//!   Citation: https://docs.rs/memmap2 — memmap best practices

use surp_core::block::BlockWriter;
use surp_core::error::{Result, SurpError};
use surp_core::wire::BlockType;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Async writer that frames Surp data into blocks over a Tokio stream.
pub struct FramedWriter<W: AsyncWrite + Unpin> {
    writer: W,
}

impl<W: AsyncWrite + Unpin> FramedWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a pre-built block.
    pub async fn write_block(&mut self, block: &[u8]) -> Result<()> {
        self.writer.write_all(block).await?;
        Ok(())
    }

    /// Write a raw data payload as a framed block.
    pub async fn write_data(&mut self, payload: &[u8]) -> Result<()> {
        let mut bw = BlockWriter::new(BlockType::Data);
        bw.write(payload);
        let block_bytes = bw.finish();
        self.writer.write_all(&block_bytes).await?;
        Ok(())
    }

    /// Flush the underlying writer.
    pub async fn flush(&mut self) -> Result<()> {
        self.writer.flush().await?;
        Ok(())
    }

    /// Consume the writer and return the inner stream.
    pub fn into_inner(self) -> W {
        self.writer
    }
}

/// Async reader that reads Surp blocks from a Tokio stream.
pub struct FramedReader<R: AsyncRead + Unpin> {
    reader: R,
    #[allow(dead_code)]
    buf: Vec<u8>,
    max_block_size: usize,
}

impl<R: AsyncRead + Unpin> FramedReader<R> {
    /// Create a reader using the default resource limits
    /// (`surp_core::Limits::default().max_block_size`) as the cap on a
    /// single block's declared payload length.
    pub fn new(reader: R) -> Self {
        Self::with_max_block_size(reader, surp_core::Limits::default().max_block_size)
    }

    /// Create a reader bounded by the `max_block_size` of the given `Limits`.
    ///
    /// This keeps `FramedReader` consistent with `surp_core::Decoder`, which
    /// enforces `Limits::max_block_size` before trusting a stream-supplied
    /// length.
    pub fn with_limits(reader: R, limits: &surp_core::Limits) -> Self {
        Self::with_max_block_size(reader, limits.max_block_size)
    }

    /// Create a reader with an explicit cap (in bytes) on a single block's
    /// declared payload length.
    pub fn with_max_block_size(reader: R, max_block_size: usize) -> Self {
        Self {
            reader,
            buf: Vec::with_capacity(4096),
            max_block_size,
        }
    }

    /// Read the next block's raw bytes. Returns None at EOF.
    pub async fn read_next_block_raw(&mut self) -> Result<Option<Vec<u8>>> {
        // Read block type (1 byte).
        let mut type_buf = [0u8; 1];
        match self.reader.read_exact(&mut type_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }

        let is_trailer = type_buf[0] == BlockType::Trailer as u8;

        // Read varint length (up to 10 bytes, 1 at a time for streaming).
        let mut len_bytes = Vec::with_capacity(10);
        loop {
            let mut b = [0u8; 1];
            self.reader.read_exact(&mut b).await?;
            len_bytes.push(b[0]);
            if b[0] & 0x80 == 0 {
                break;
            }
            if len_bytes.len() > 10 {
                return Err(SurpError::VarintOverflow);
            }
        }

        let (block_len, _) = surp_core::varint::decode_varint(&len_bytes, 0)?;
        let block_len =
            usize::try_from(block_len).map_err(|_| SurpError::LengthOverflow(block_len))?;

        // Reject oversized blocks *before* allocating the payload buffer, to
        // avoid a memory-exhaustion DoS from a malicious/corrupt peer sending
        // an enormous declared length.
        if block_len > self.max_block_size {
            return Err(SurpError::BlockTooLarge(block_len, self.max_block_size));
        }

        // Read compression type (1 byte) + checksum (8 bytes) + payload.
        let remaining = 1 + 8 + block_len;
        let mut payload = vec![0u8; remaining];
        self.reader.read_exact(&mut payload).await?;

        if is_trailer {
            // Fully consume the trailer's bytes so a reused `FramedReader`
            // (multi-document stream) stays in sync, but signal EOF to the
            // caller as before.
            return Ok(None);
        }

        // Reconstruct the full block bytes.
        let mut block = Vec::with_capacity(1 + len_bytes.len() + remaining);
        block.push(type_buf[0]);
        block.extend_from_slice(&len_bytes);
        block.extend_from_slice(&payload);

        Ok(Some(block))
    }
}

/// Read a complete Surp file from memory-mapped or in-memory bytes.
///
/// This is the simplest API for reading a complete file.
pub fn read_file_bytes(data: &[u8]) -> Result<Vec<surp_core::Value>> {
    let mut decoder = surp_core::Decoder::new(data);
    decoder.decode_all_owned()
}

/// Write values to an in-memory buffer as a complete Surp file.
pub fn write_values_to_bytes(values: &[surp_core::Value]) -> Result<Vec<u8>> {
    let mut encoder = surp_core::Encoder::new();
    for v in values {
        encoder.encode_value(v)?;
    }
    encoder.finish()
}

// ---------------------------------------------------------------------------
// Bytes-based shared buffer API
// ---------------------------------------------------------------------------

/// Read a complete Surp file from a `bytes::Bytes` buffer.
///
/// The `Bytes` reference-counted buffer avoids copies when sharing
/// between threads or network layers.
/// Citation: https://docs.rs/bytes
pub fn read_from_shared(data: bytes::Bytes) -> Result<Vec<surp_core::Value>> {
    let mut decoder = surp_core::Decoder::new(&data);
    decoder.decode_all_owned()
}

/// Write values into a `bytes::Bytes` shared buffer.
pub fn write_to_shared(values: &[surp_core::Value]) -> Result<bytes::Bytes> {
    let vec = write_values_to_bytes(values)?;
    Ok(bytes::Bytes::from(vec))
}

// ---------------------------------------------------------------------------
// Memory-mapped file reader (feature = "mmap")
// ---------------------------------------------------------------------------

/// Zero-copy memory-mapped file reader for Surp files.
///
/// Maps a file into the process address space and provides direct
/// zero-copy access to the underlying bytes. The `Decoder` can
/// borrow `SurpValue<'a>` directly from the mapped memory.
///
/// # Safety considerations
/// The file must not be modified while the mapping is live.
/// `MmapReader` uses a read-only mapping which will cause SIGBUS
/// if the file is truncated. For untrusted files, prefer `read_file_bytes`.
///
/// Citation: memmap best practices — https://docs.rs/memmap2
#[cfg(feature = "mmap")]
pub struct MmapReader {
    _mmap: memmap2::Mmap,
}

#[cfg(feature = "mmap")]
impl MmapReader {
    /// Open a Surp file for zero-copy reading.
    ///
    /// ```rust,ignore
    /// let reader = MmapReader::open("data.surp")?;
    /// let values = reader.decode_all()?;
    /// ```
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        // SAFETY: We require the file not to be modified while mapped.
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        Ok(Self { _mmap: mmap })
    }

    /// Get a reference to the mapped bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self._mmap
    }

    /// Get the file size.
    pub fn len(&self) -> usize {
        self._mmap.len()
    }

    /// Check if the mapping is empty.
    pub fn is_empty(&self) -> bool {
        self._mmap.is_empty()
    }

    /// Create a decoder over the mapped memory.
    ///
    /// The returned decoder borrows from the mapping, enabling zero-copy
    /// `SurpValue<'_>` decoding with no additional allocation for strings/bytes.
    pub fn decoder(&self) -> surp_core::Decoder<'_> {
        surp_core::Decoder::new(&self._mmap)
    }

    /// Create a decoder with custom limits.
    pub fn decoder_with_limits(&self, limits: surp_core::Limits) -> surp_core::Decoder<'_> {
        surp_core::Decoder::with_limits(&self._mmap, limits)
    }

    /// Convenience: decode all values as owned Values.
    pub fn decode_all(&self) -> Result<Vec<surp_core::Value>> {
        let mut dec = self.decoder();
        dec.decode_all_owned()
    }

    /// Convenience: decode all values as zero-copy SurpValues.
    pub fn decode_all_borrowed(&self) -> Result<Vec<surp_core::SurpValue<'_>>> {
        let mut dec = self.decoder();
        dec.decode_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surp_core::Value;

    #[tokio::test]
    async fn framed_writer_basic() {
        let mut buf = Vec::new();
        {
            let mut writer = FramedWriter::new(&mut buf);
            writer.write_data(b"hello").await.unwrap();
            writer.flush().await.unwrap();
        }
        assert_eq!(buf[0], BlockType::Data as u8);
    }

    #[test]
    fn read_write_bytes() {
        let values = vec![Value::Str("hello".into()), Value::UInt(42)];
        let bytes = write_values_to_bytes(&values).unwrap();
        let decoded = read_file_bytes(&bytes).unwrap();
        assert_eq!(decoded, values);
    }

    #[tokio::test]
    async fn framed_reader_round_trip() {
        let mut buf = Vec::new();
        {
            let mut writer = FramedWriter::new(&mut buf);
            writer.write_data(b"hello").await.unwrap();
            writer.write_data(b"world").await.unwrap();
            writer.flush().await.unwrap();
        }

        let mut reader = FramedReader::new(buf.as_slice());
        let block1 = reader.read_next_block_raw().await.unwrap().unwrap();
        assert_eq!(block1[0], BlockType::Data as u8);
        let block2 = reader.read_next_block_raw().await.unwrap().unwrap();
        assert_eq!(block2[0], BlockType::Data as u8);
        assert_ne!(block1, block2);

        // Stream is exhausted (no trailer was written here, so this hits EOF).
        let end = reader.read_next_block_raw().await.unwrap();
        assert!(end.is_none());
    }

    #[tokio::test]
    async fn framed_reader_rejects_oversized_block_before_allocating() {
        // Build a stream declaring a block length far larger than our cap,
        // without ever supplying the (huge amount of) trailing bytes. If the
        // reader tried to allocate/read the declared length first, this
        // would hang waiting on `read_exact` or attempt a huge allocation
        // instead of failing fast.
        let mut stream = Vec::new();
        stream.push(BlockType::Data as u8);
        // Declare an obviously-oversized block length.
        surp_core::varint::encode_varint_vec(4 * 1024 * 1024 * 1024, &mut stream);
        // Intentionally do NOT append compression byte / checksum / payload.

        let max_block_size = 1024; // 1 KiB cap, far below the declared length.
        let mut reader = FramedReader::with_max_block_size(stream.as_slice(), max_block_size);

        let result = reader.read_next_block_raw().await;
        assert!(
            matches!(
                result,
                Err(SurpError::BlockTooLarge(_, cap)) if cap == max_block_size
            ),
            "expected BlockTooLarge error, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn framed_reader_accepts_block_within_custom_cap() {
        let mut buf = Vec::new();
        {
            let mut writer = FramedWriter::new(&mut buf);
            writer.write_data(b"small payload").await.unwrap();
            writer.flush().await.unwrap();
        }

        let mut reader = FramedReader::with_max_block_size(buf.as_slice(), 4096);
        let block = reader.read_next_block_raw().await.unwrap().unwrap();
        assert_eq!(block[0], BlockType::Data as u8);
    }
}
