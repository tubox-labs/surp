//! RFC-001 CBF (Surp Binary Format) encoder/decoder.
//!
//! This module implements a segment-tree binary representation aligned with
//! RFC-001 principles while preserving the existing v1 API untouched.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::error::{Result, SurpError};
use crate::varint::{
    decode_signed_varint, decode_varint, encode_signed_varint_vec, encode_varint_vec,
};

use super::ast::{
    Annotation, Document, Field, Opaque, Product, Reference, Scalar, Sequence, Stream, Sum,
    SumPayload, Tensor, TensorData, Value,
};

pub const CBF_MAGIC: [u8; 4] = *b"SURP";
pub const CBF_HEADER_SIZE: usize = 32;

const FLAG_SELF_DESCRIBING: u8 = 1 << 0;
const FLAG_HAS_SYMTAB: u8 = 1 << 1;
const FLAG_HAS_INDEX: u8 = 1 << 2;

const TYPE_PRIMITIVE: u8 = 0x0;
const TYPE_STRING: u8 = 0x1;
const TYPE_BYTES: u8 = 0x2;
const TYPE_SYMBOL: u8 = 0x3;
const TYPE_STRUCT: u8 = 0x4;
const TYPE_ENUM: u8 = 0x5;
const TYPE_SEQUENCE: u8 = 0x6;
const TYPE_MAP: u8 = 0x7;
const TYPE_TENSOR: u8 = 0x8;
const TYPE_REFERENCE: u8 = 0x9;
const TYPE_STREAM: u8 = 0xA;
const TYPE_ANNOTATION: u8 = 0xB;
const TYPE_OPAQUE: u8 = 0xD;
const TYPE_SPECIAL: u8 = 0xF;

const SPECIAL_NULL: u8 = 0x0;
const SPECIAL_UNIT: u8 = 0x1;
const SPECIAL_TRUE: u8 = 0x2;
const SPECIAL_FALSE: u8 = 0x3;
const SPECIAL_EMPTY_SEQUENCE: u8 = 0x4;
const SPECIAL_EMPTY_MAP: u8 = 0x5;
const SPECIAL_EMPTY_STRING: u8 = 0x6;
const SPECIAL_EMPTY_BYTES: u8 = 0x7;

const PRIM_I64: u8 = 0x3;
const PRIM_U64: u8 = 0x8;
const PRIM_VI64: u8 = 0xA;
const PRIM_VU64: u8 = 0xB;
const PRIM_F32: u8 = 0xC;
const PRIM_F64: u8 = 0xD;

const SEQ_HAS_OFFSET_TABLE: u8 = 0b1000;
const MAP_HAS_OFFSET_TABLE: u8 = 0b1000;

/// CBF file header (32 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CbfHeader {
    pub cbf_version: u8,
    pub ctn_version: u8,
    pub flags: u8,
    pub alignment: u8,
    pub schema_hash_prefix: [u8; 8],
    pub root_offset: u64,
    pub symtab_offset: u32,
    pub index_offset: u32,
}

impl CbfHeader {
    pub fn new() -> Self {
        Self {
            cbf_version: 0x01,
            ctn_version: 0x01,
            flags: FLAG_SELF_DESCRIBING,
            alignment: 0,
            schema_hash_prefix: [0u8; 8],
            root_offset: 0,
            symtab_offset: 0,
            index_offset: 0,
        }
    }

    pub fn has_symtab(&self) -> bool {
        self.flags & FLAG_HAS_SYMTAB != 0
    }

    pub fn has_index(&self) -> bool {
        self.flags & FLAG_HAS_INDEX != 0
    }

    pub fn self_describing(&self) -> bool {
        self.flags & FLAG_SELF_DESCRIBING != 0
    }

    pub fn encode(&self) -> [u8; CBF_HEADER_SIZE] {
        let mut out = [0u8; CBF_HEADER_SIZE];
        out[..4].copy_from_slice(&CBF_MAGIC);
        out[4] = self.cbf_version;
        out[5] = self.ctn_version;
        out[6] = self.flags;
        out[7] = self.alignment;
        out[8..16].copy_from_slice(&self.schema_hash_prefix);
        out[16..24].copy_from_slice(&self.root_offset.to_le_bytes());
        out[24..28].copy_from_slice(&self.symtab_offset.to_le_bytes());
        out[28..32].copy_from_slice(&self.index_offset.to_le_bytes());
        out
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < CBF_HEADER_SIZE {
            return Err(SurpError::UnexpectedEof(data.len()));
        }
        if data[..4] != CBF_MAGIC {
            return Err(SurpError::InvalidMagic);
        }

        let cbf_version = data[4];
        if cbf_version != 0x01 {
            return Err(SurpError::UnsupportedVersion(cbf_version));
        }

        let ctn_version = data[5];
        let flags = data[6];
        let alignment = data[7];
        let mut schema_hash_prefix = [0u8; 8];
        schema_hash_prefix.copy_from_slice(&data[8..16]);

        let root_offset = u64::from_le_bytes(
            data[16..24]
                .try_into()
                .expect("header root offset bytes length"),
        );
        let symtab_offset = u32::from_le_bytes(
            data[24..28]
                .try_into()
                .expect("header symtab offset bytes length"),
        );
        let index_offset = u32::from_le_bytes(
            data[28..32]
                .try_into()
                .expect("header index offset bytes length"),
        );

        Ok(Self {
            cbf_version,
            ctn_version,
            flags,
            alignment,
            schema_hash_prefix,
            root_offset,
            symtab_offset,
            index_offset,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EncodeOptions {
    /// Build and emit a symbol table for symbol literals and field names.
    pub with_symtab: bool,
    /// Alignment hint encoded into the file header.
    pub alignment: u8,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            with_symtab: true,
            alignment: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecodedDocument {
    pub header: CbfHeader,
    pub symbols: Vec<String>,
    pub document: Document,
}

/// Encode an RFC-001 document into CBF bytes.
pub fn encode_document(doc: &Document, options: EncodeOptions) -> Result<Vec<u8>> {
    let root = resolve_references(&doc.effective_root()?, doc, &mut HashSet::new())?;

    let mut symbols = BTreeSet::new();
    collect_symbols(&root, &mut symbols);

    let symbol_list: Vec<String> = if options.with_symtab {
        symbols.into_iter().collect()
    } else {
        if contains_symbol_value(&root) {
            return Err(SurpError::InvalidData(
                "symbol values require symbol table; enable EncodeOptions::with_symtab".into(),
            ));
        }
        Vec::new()
    };

    let mut symbol_index = HashMap::new();
    for (idx, sym) in symbol_list.iter().enumerate() {
        symbol_index.insert(sym.clone(), idx as u32);
    }

    let symtab_bytes = if symbol_list.is_empty() {
        Vec::new()
    } else {
        encode_symbol_table(&symbol_list)
    };

    let mut payload = Vec::new();
    payload.extend_from_slice(&symtab_bytes);

    let root_offset = payload.len() as u64;
    encode_segment(
        &root,
        &EncodeCtx {
            symbols: symbol_index,
            has_symtab: !symbol_list.is_empty(),
        },
        &mut payload,
    )?;

    let mut header = CbfHeader::new();
    header.alignment = options.alignment;
    header.root_offset = root_offset;
    if !symbol_list.is_empty() {
        header.flags |= FLAG_HAS_SYMTAB;
        header.symtab_offset = 0;
    }

    let mut out = Vec::with_capacity(CBF_HEADER_SIZE + payload.len() + 8);
    out.extend_from_slice(&header.encode());
    out.extend_from_slice(&payload);

    let checksum = crc64_ecma(&out);
    out.extend_from_slice(&checksum.to_le_bytes());
    Ok(out)
}

/// Decode a CBF document.
pub fn decode_document(data: &[u8]) -> Result<DecodedDocument> {
    if data.len() < CBF_HEADER_SIZE + 8 {
        return Err(SurpError::UnexpectedEof(data.len()));
    }

    let header = CbfHeader::decode(data)?;

    let expected_checksum = u64::from_le_bytes(
        data[data.len() - 8..]
            .try_into()
            .expect("checksum suffix length"),
    );
    let actual_checksum = crc64_ecma(&data[..data.len() - 8]);
    if expected_checksum != actual_checksum {
        return Err(SurpError::ChecksumMismatch {
            expected: expected_checksum,
            actual: actual_checksum,
        });
    }

    let payload = &data[CBF_HEADER_SIZE..data.len() - 8];

    let symbols = if header.has_symtab() {
        let start = usize::try_from(header.symtab_offset)
            .map_err(|_| SurpError::LengthOverflow(header.symtab_offset as u64))?;
        decode_symbol_table(payload, start)?
    } else {
        Vec::new()
    };

    let root_offset = usize::try_from(header.root_offset)
        .map_err(|_| SurpError::LengthOverflow(header.root_offset))?;
    if root_offset >= payload.len() {
        return Err(SurpError::UnexpectedEof(root_offset));
    }

    let (root, consumed) = decode_segment(payload, root_offset, &symbols, 0)?;
    if root_offset + consumed > payload.len() {
        return Err(SurpError::UnexpectedEof(root_offset + consumed));
    }

    let document = Document {
        root: Some(root),
        ..Document::default()
    };

    Ok(DecodedDocument {
        header,
        symbols,
        document,
    })
}

/// Convenience encode for a single root value.
pub fn encode_value(value: &Value, options: EncodeOptions) -> Result<Vec<u8>> {
    let doc = Document {
        root: Some(value.clone()),
        ..Document::default()
    };
    encode_document(&doc, options)
}

/// Convenience decode for a single root value.
pub fn decode_value(data: &[u8]) -> Result<Value> {
    let decoded = decode_document(data)?;
    decoded.document.effective_root()
}

#[derive(Debug)]
struct SegmentHeader {
    type_nibble: u8,
    config_nibble: u8,
    payload_len: usize,
    header_len: usize,
}

fn parse_segment_header(data: &[u8], offset: usize) -> Result<SegmentHeader> {
    if offset + 4 > data.len() {
        return Err(SurpError::UnexpectedEof(offset));
    }

    let first = data[offset];
    let type_nibble = first >> 4;
    let config_nibble = first & 0x0F;

    let mut len = usize::from(data[offset + 1])
        | (usize::from(data[offset + 2]) << 8)
        | (usize::from(data[offset + 3]) << 16);

    let mut header_len = 4;
    if len == 0xFF_FFFF {
        if offset + 12 > data.len() {
            return Err(SurpError::UnexpectedEof(offset + 4));
        }
        let big_len = u64::from_le_bytes(
            data[offset + 4..offset + 12]
                .try_into()
                .expect("extended length bytes"),
        );
        len = usize::try_from(big_len).map_err(|_| SurpError::LengthOverflow(big_len))?;
        header_len = 12;
    }

    Ok(SegmentHeader {
        type_nibble,
        config_nibble,
        payload_len: len,
        header_len,
    })
}

fn write_segment_header(
    out: &mut Vec<u8>,
    type_nibble: u8,
    config_nibble: u8,
    payload_len: usize,
) -> Result<()> {
    if type_nibble > 0x0F || config_nibble > 0x0F {
        return Err(SurpError::InvalidData(
            "invalid segment header nibble".into(),
        ));
    }

    out.push((type_nibble << 4) | (config_nibble & 0x0F));

    if payload_len <= 0xFF_FFFF {
        out.push((payload_len & 0xFF) as u8);
        out.push(((payload_len >> 8) & 0xFF) as u8);
        out.push(((payload_len >> 16) & 0xFF) as u8);
    } else {
        out.push(0xFF);
        out.push(0xFF);
        out.push(0xFF);
        out.extend_from_slice(&(payload_len as u64).to_le_bytes());
    }

    Ok(())
}

#[derive(Debug)]
struct EncodeCtx {
    symbols: HashMap<String, u32>,
    has_symtab: bool,
}

fn encode_segment(value: &Value, ctx: &EncodeCtx, out: &mut Vec<u8>) -> Result<()> {
    let mut payload = Vec::new();
    let (ty, cfg) = match value {
        Value::Scalar(Scalar::Null) => (TYPE_SPECIAL, SPECIAL_NULL),
        Value::Scalar(Scalar::Unit) => (TYPE_SPECIAL, SPECIAL_UNIT),
        Value::Scalar(Scalar::Bool(true)) => (TYPE_SPECIAL, SPECIAL_TRUE),
        Value::Scalar(Scalar::Bool(false)) => (TYPE_SPECIAL, SPECIAL_FALSE),
        Value::Scalar(Scalar::Str(s)) => {
            if s.is_empty() {
                (TYPE_SPECIAL, SPECIAL_EMPTY_STRING)
            } else {
                payload.extend_from_slice(s.as_bytes());
                (TYPE_STRING, 0)
            }
        }
        Value::Scalar(Scalar::Bytes(b)) => {
            if b.is_empty() {
                (TYPE_SPECIAL, SPECIAL_EMPTY_BYTES)
            } else {
                payload.extend_from_slice(b);
                (TYPE_BYTES, 0)
            }
        }
        Value::Scalar(Scalar::Sym(sym)) => {
            if !ctx.has_symtab {
                return Err(SurpError::InvalidData(
                    "symbol encoding requires symbol table".into(),
                ));
            }
            let idx = ctx.symbols.get(sym).ok_or_else(|| {
                SurpError::InvalidData(format!("missing symbol table entry '{sym}'"))
            })?;
            encode_varint_vec(u64::from(*idx), &mut payload);
            (TYPE_SYMBOL, 0)
        }
        Value::Scalar(Scalar::Tagged { tag, value }) => {
            encode_varint_vec(tag.len() as u64, &mut payload);
            payload.extend_from_slice(tag.as_bytes());
            encode_varint_vec(value.len() as u64, &mut payload);
            payload.extend_from_slice(value.as_bytes());
            (TYPE_OPAQUE, 0)
        }
        Value::Scalar(Scalar::I64(v)) => {
            payload.extend_from_slice(&v.to_le_bytes());
            (TYPE_PRIMITIVE, PRIM_I64)
        }
        Value::Scalar(Scalar::U64(v)) => {
            payload.extend_from_slice(&v.to_le_bytes());
            (TYPE_PRIMITIVE, PRIM_U64)
        }
        Value::Scalar(Scalar::Vi64(v)) => {
            encode_signed_varint_vec(*v, &mut payload);
            (TYPE_PRIMITIVE, PRIM_VI64)
        }
        Value::Scalar(Scalar::Vu64(v)) => {
            encode_varint_vec(*v, &mut payload);
            (TYPE_PRIMITIVE, PRIM_VU64)
        }
        Value::Scalar(Scalar::F32(v)) => {
            payload.extend_from_slice(&v.to_le_bytes());
            (TYPE_PRIMITIVE, PRIM_F32)
        }
        Value::Scalar(Scalar::F64(v)) => {
            payload.extend_from_slice(&v.to_le_bytes());
            (TYPE_PRIMITIVE, PRIM_F64)
        }
        Value::Product(product) => {
            encode_product(product, ctx, &mut payload)?;
            (TYPE_STRUCT, 0)
        }
        Value::Sum(sum) => {
            encode_sum(sum, ctx, &mut payload)?;
            (TYPE_ENUM, 0)
        }
        Value::Sequence(seq) => {
            if seq.items.is_empty() {
                (TYPE_SPECIAL, SPECIAL_EMPTY_SEQUENCE)
            } else {
                encode_sequence(seq, ctx, &mut payload)?;
                (TYPE_SEQUENCE, SEQ_HAS_OFFSET_TABLE)
            }
        }
        Value::Association(map) => {
            if map.is_empty() {
                (TYPE_SPECIAL, SPECIAL_EMPTY_MAP)
            } else {
                encode_map(map, ctx, &mut payload)?;
                (TYPE_MAP, MAP_HAS_OFFSET_TABLE)
            }
        }
        Value::Reference(reference) => {
            encode_reference(reference, ctx, &mut payload)?;
            (TYPE_REFERENCE, 0)
        }
        Value::Tensor(tensor) => {
            encode_tensor(tensor, &mut payload)?;
            (TYPE_TENSOR, 0)
        }
        Value::Stream(stream) => {
            encode_stream(stream, ctx, &mut payload)?;
            (TYPE_STREAM, 0)
        }
        Value::Opaque(opaque) => {
            encode_varint_vec(opaque.type_tag.len() as u64, &mut payload);
            payload.extend_from_slice(opaque.type_tag.as_bytes());
            encode_varint_vec(opaque.bytes.len() as u64, &mut payload);
            payload.extend_from_slice(&opaque.bytes);
            (TYPE_OPAQUE, 1)
        }
    };

    write_segment_header(out, ty, cfg, payload.len())?;
    out.extend_from_slice(&payload);
    Ok(())
}

fn encode_product(product: &Product, ctx: &EncodeCtx, payload: &mut Vec<u8>) -> Result<()> {
    match &product.type_name {
        Some(name) => {
            payload.push(1);
            encode_segment(&Value::Scalar(Scalar::Str(name.clone())), ctx, payload)?;
        }
        None => payload.push(0),
    }

    encode_varint_vec(product.fields.len() as u64, payload);
    for field in &product.fields {
        encode_field_name(&field.name, ctx, payload)?;
        encode_segment(&field.value, ctx, payload)?;
    }

    Ok(())
}

fn encode_sum(sum: &Sum, ctx: &EncodeCtx, payload: &mut Vec<u8>) -> Result<()> {
    match &sum.type_name {
        Some(name) => {
            payload.push(1);
            encode_segment(&Value::Scalar(Scalar::Str(name.clone())), ctx, payload)?;
        }
        None => payload.push(0),
    }

    encode_segment(
        &Value::Scalar(Scalar::Str(sum.variant.clone())),
        ctx,
        payload,
    )?;

    match &sum.payload {
        SumPayload::Unit => payload.push(0),
        SumPayload::Tuple(items) => {
            payload.push(1);
            encode_varint_vec(items.len() as u64, payload);
            for item in items {
                encode_segment(item, ctx, payload)?;
            }
        }
        SumPayload::Struct(fields) => {
            payload.push(2);
            encode_varint_vec(fields.len() as u64, payload);
            for field in fields {
                encode_field_name(&field.name, ctx, payload)?;
                encode_segment(&field.value, ctx, payload)?;
            }
        }
    }

    Ok(())
}

fn encode_sequence(seq: &Sequence, ctx: &EncodeCtx, payload: &mut Vec<u8>) -> Result<()> {
    match &seq.elem_type {
        Some(elem_type) => {
            payload.push(1);
            encode_segment(&Value::Scalar(Scalar::Str(elem_type.clone())), ctx, payload)?;
        }
        None => payload.push(0),
    }

    encode_varint_vec(seq.items.len() as u64, payload);

    let mut encoded_items = Vec::with_capacity(seq.items.len());
    for item in &seq.items {
        let mut buf = Vec::new();
        encode_segment(item, ctx, &mut buf)?;
        encoded_items.push(buf);
    }

    let table_len = encoded_items.len() * 4;
    let mut running = 0usize;
    for item in &encoded_items {
        let off = u32::try_from(running).map_err(|_| {
            SurpError::InvalidData("sequence segment payload exceeds u32 offsets".into())
        })?;
        payload.extend_from_slice(&off.to_le_bytes());
        running = running.saturating_add(item.len());
    }

    debug_assert_eq!(
        payload.len() - table_len,
        payload.len() - encoded_items.len() * 4
    );

    for item in &encoded_items {
        payload.extend_from_slice(item);
    }

    Ok(())
}

fn encode_map(map: &[(Value, Value)], ctx: &EncodeCtx, payload: &mut Vec<u8>) -> Result<()> {
    encode_varint_vec(map.len() as u64, payload);

    let mut encoded_pairs = Vec::with_capacity(map.len());
    for (key, value) in map {
        let mut buf = Vec::new();
        encode_segment(key, ctx, &mut buf)?;
        encode_segment(value, ctx, &mut buf)?;
        encoded_pairs.push(buf);
    }

    let mut running = 0usize;
    for pair in &encoded_pairs {
        let off = u32::try_from(running).map_err(|_| {
            SurpError::InvalidData("map segment payload exceeds u32 offsets".into())
        })?;
        payload.extend_from_slice(&off.to_le_bytes());
        running = running.saturating_add(pair.len());
    }

    for pair in &encoded_pairs {
        payload.extend_from_slice(pair);
    }

    Ok(())
}

fn encode_reference(reference: &Reference, ctx: &EncodeCtx, payload: &mut Vec<u8>) -> Result<()> {
    match reference {
        Reference::Binding(name) => {
            payload.push(0);
            encode_varint_vec(name.len() as u64, payload);
            payload.extend_from_slice(name.as_bytes());
        }
        Reference::ById(id) => {
            payload.push(1);
            encode_segment(id, ctx, payload)?;
        }
    }
    Ok(())
}

fn encode_tensor(tensor: &Tensor, payload: &mut Vec<u8>) -> Result<()> {
    payload.push(tensor_element_type_code(&tensor.element_type));
    payload.push(
        u8::try_from(tensor.shape.len())
            .map_err(|_| SurpError::InvalidData("tensor dimensionality exceeds u8".into()))?,
    );
    payload.push(0x01); // row_major
    payload.push(0);

    for dim in &tensor.shape {
        match dim {
            Some(v) => encode_varint_vec(*v, payload),
            None => encode_varint_vec(0, payload),
        }
    }

    match &tensor.data {
        TensorData::DenseF64(values) => {
            payload.extend_from_slice(&(values.len() as u64).to_le_bytes());
            for value in values {
                payload.extend_from_slice(&value.to_le_bytes());
            }
        }
        TensorData::DenseI64(values) => {
            payload.extend_from_slice(&(values.len() as u64).to_le_bytes());
            for value in values {
                payload.extend_from_slice(&value.to_le_bytes());
            }
        }
        TensorData::DenseU64(values) => {
            payload.extend_from_slice(&(values.len() as u64).to_le_bytes());
            for value in values {
                payload.extend_from_slice(&value.to_le_bytes());
            }
        }
        TensorData::BinaryBlob(bytes) => {
            payload.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            payload.extend_from_slice(bytes);
        }
    }

    Ok(())
}

fn encode_stream(stream: &Stream, ctx: &EncodeCtx, payload: &mut Vec<u8>) -> Result<()> {
    encode_segment(
        &Value::Scalar(Scalar::Str(stream.item_type.clone())),
        ctx,
        payload,
    )?;

    encode_varint_vec(stream.annotations.len() as u64, payload);
    for ann in &stream.annotations {
        encode_annotation(ann, ctx, payload)?;
    }

    Ok(())
}

fn encode_annotation(ann: &Annotation, ctx: &EncodeCtx, payload: &mut Vec<u8>) -> Result<()> {
    encode_segment(&Value::Scalar(Scalar::Str(ann.name.clone())), ctx, payload)?;
    match &ann.value {
        Some(v) => {
            payload.push(1);
            encode_segment(&Value::Scalar(v.clone()), ctx, payload)?;
        }
        None => payload.push(0),
    }
    Ok(())
}

fn encode_field_name(name: &str, ctx: &EncodeCtx, payload: &mut Vec<u8>) -> Result<()> {
    if ctx.has_symtab {
        let idx = ctx.symbols.get(name).ok_or_else(|| {
            SurpError::InvalidData(format!("missing symbol table entry for field '{name}'"))
        })?;
        encode_segment(&Value::Scalar(Scalar::Sym(name.to_string())), ctx, payload)?;
        debug_assert_eq!(ctx.symbols.get(name), Some(idx));
        Ok(())
    } else {
        encode_segment(&Value::Scalar(Scalar::Str(name.to_string())), ctx, payload)
    }
}

fn decode_segment(
    data: &[u8],
    offset: usize,
    symbols: &[String],
    depth: usize,
) -> Result<(Value, usize)> {
    if depth > 64 {
        return Err(SurpError::NestingTooDeep(depth, 64));
    }

    let header = parse_segment_header(data, offset)?;
    let payload_start = offset + header.header_len;
    let payload_end = payload_start
        .checked_add(header.payload_len)
        .ok_or(SurpError::LengthOverflow(header.payload_len as u64))?;

    if payload_end > data.len() {
        return Err(SurpError::UnexpectedEof(payload_end));
    }

    let payload = &data[payload_start..payload_end];

    let value = match (header.type_nibble, header.config_nibble) {
        (TYPE_SPECIAL, SPECIAL_NULL) => Value::Scalar(Scalar::Null),
        (TYPE_SPECIAL, SPECIAL_UNIT) => Value::Scalar(Scalar::Unit),
        (TYPE_SPECIAL, SPECIAL_TRUE) => Value::Scalar(Scalar::Bool(true)),
        (TYPE_SPECIAL, SPECIAL_FALSE) => Value::Scalar(Scalar::Bool(false)),
        (TYPE_SPECIAL, SPECIAL_EMPTY_STRING) => Value::Scalar(Scalar::Str(String::new())),
        (TYPE_SPECIAL, SPECIAL_EMPTY_BYTES) => Value::Scalar(Scalar::Bytes(Vec::new())),
        (TYPE_SPECIAL, SPECIAL_EMPTY_SEQUENCE) => Value::Sequence(Sequence {
            elem_type: None,
            items: Vec::new(),
        }),
        (TYPE_SPECIAL, SPECIAL_EMPTY_MAP) => Value::Association(Vec::new()),
        (TYPE_STRING, _) => {
            let s = std::str::from_utf8(payload)
                .map_err(|_| SurpError::InvalidUtf8(payload_start))?
                .to_string();
            Value::Scalar(Scalar::Str(s))
        }
        (TYPE_BYTES, _) => Value::Scalar(Scalar::Bytes(payload.to_vec())),
        (TYPE_SYMBOL, _) => {
            let (idx, consumed) = decode_varint(payload, 0)?;
            if consumed != payload.len() {
                return Err(SurpError::InvalidData(
                    "symbol segment has trailing payload".into(),
                ));
            }
            let idx = usize::try_from(idx).map_err(|_| SurpError::LengthOverflow(idx))?;
            let sym = symbols
                .get(idx)
                .ok_or(SurpError::InvalidReference(idx, symbols.len()))?;
            Value::Scalar(Scalar::Sym(sym.clone()))
        }
        (TYPE_PRIMITIVE, PRIM_I64) => {
            if payload.len() != 8 {
                return Err(SurpError::InvalidData(
                    "invalid i64 primitive payload size".into(),
                ));
            }
            Value::Scalar(Scalar::I64(i64::from_le_bytes(
                payload.try_into().expect("i64 payload"),
            )))
        }
        (TYPE_PRIMITIVE, PRIM_U64) => {
            if payload.len() != 8 {
                return Err(SurpError::InvalidData(
                    "invalid u64 primitive payload size".into(),
                ));
            }
            Value::Scalar(Scalar::U64(u64::from_le_bytes(
                payload.try_into().expect("u64 payload"),
            )))
        }
        (TYPE_PRIMITIVE, PRIM_VI64) => {
            let (value, consumed) = decode_signed_varint(payload, 0)?;
            if consumed != payload.len() {
                return Err(SurpError::InvalidData(
                    "varint primitive trailing bytes".into(),
                ));
            }
            Value::Scalar(Scalar::Vi64(value))
        }
        (TYPE_PRIMITIVE, PRIM_VU64) => {
            let (value, consumed) = decode_varint(payload, 0)?;
            if consumed != payload.len() {
                return Err(SurpError::InvalidData(
                    "varint primitive trailing bytes".into(),
                ));
            }
            Value::Scalar(Scalar::Vu64(value))
        }
        (TYPE_PRIMITIVE, PRIM_F32) => {
            if payload.len() != 4 {
                return Err(SurpError::InvalidData(
                    "invalid f32 primitive payload size".into(),
                ));
            }
            Value::Scalar(Scalar::F32(f32::from_le_bytes(
                payload.try_into().expect("f32 payload"),
            )))
        }
        (TYPE_PRIMITIVE, PRIM_F64) => {
            if payload.len() != 8 {
                return Err(SurpError::InvalidData(
                    "invalid f64 primitive payload size".into(),
                ));
            }
            Value::Scalar(Scalar::F64(f64::from_le_bytes(
                payload.try_into().expect("f64 payload"),
            )))
        }
        (TYPE_STRUCT, _) => decode_product(payload, symbols, depth + 1)?,
        (TYPE_ENUM, _) => decode_sum(payload, symbols, depth + 1)?,
        (TYPE_SEQUENCE, cfg) => decode_sequence(payload, symbols, depth + 1, cfg)?,
        (TYPE_MAP, cfg) => decode_map(payload, symbols, depth + 1, cfg)?,
        (TYPE_REFERENCE, _) => decode_reference(payload, symbols, depth + 1)?,
        (TYPE_TENSOR, _) => decode_tensor(payload)?,
        (TYPE_STREAM, _) => decode_stream(payload, symbols, depth + 1)?,
        (TYPE_OPAQUE, cfg) => decode_opaque(payload, cfg)?,
        (TYPE_ANNOTATION, _) => {
            let annotation = decode_annotation(payload, symbols, depth + 1)?;
            Value::Opaque(Opaque {
                type_tag: "annotation".into(),
                bytes: format!("{}={}", annotation.name, annotation.value.is_some()).into_bytes(),
            })
        }
        _ => {
            return Err(SurpError::InvalidData(format!(
                "unsupported segment type/config: 0x{:x}/0x{:x}",
                header.type_nibble, header.config_nibble
            )));
        }
    };

    Ok((value, header.header_len + header.payload_len))
}

fn decode_product(payload: &[u8], symbols: &[String], depth: usize) -> Result<Value> {
    let mut pos = 0usize;

    if pos >= payload.len() {
        return Err(SurpError::UnexpectedEof(pos));
    }

    let has_type = payload[pos];
    pos += 1;

    let type_name = if has_type == 1 {
        let (value, consumed) = decode_segment(payload, pos, symbols, depth)?;
        pos += consumed;
        Some(expect_string_scalar(&value, "struct type name")?)
    } else {
        None
    };

    let (field_count, consumed) = decode_varint(payload, pos)?;
    pos += consumed;
    let field_count =
        usize::try_from(field_count).map_err(|_| SurpError::LengthOverflow(field_count))?;

    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        let (name_segment, consumed_name) = decode_segment(payload, pos, symbols, depth)?;
        pos += consumed_name;
        let name = expect_field_name(&name_segment)?;

        let (value, consumed_value) = decode_segment(payload, pos, symbols, depth)?;
        pos += consumed_value;

        fields.push(Field { name, value });
    }

    if pos != payload.len() {
        return Err(SurpError::InvalidData(
            "struct payload has trailing bytes".into(),
        ));
    }

    Ok(Value::Product(Product { type_name, fields }))
}

fn decode_sum(payload: &[u8], symbols: &[String], depth: usize) -> Result<Value> {
    let mut pos = 0usize;
    if pos >= payload.len() {
        return Err(SurpError::UnexpectedEof(pos));
    }

    let has_type = payload[pos];
    pos += 1;

    let type_name = if has_type == 1 {
        let (value, consumed) = decode_segment(payload, pos, symbols, depth)?;
        pos += consumed;
        Some(expect_string_scalar(&value, "enum type name")?)
    } else {
        None
    };

    let (variant_value, consumed_variant) = decode_segment(payload, pos, symbols, depth)?;
    pos += consumed_variant;
    let variant = expect_string_scalar(&variant_value, "enum variant")?;

    if pos >= payload.len() {
        return Err(SurpError::UnexpectedEof(pos));
    }
    let payload_kind = payload[pos];
    pos += 1;

    let variant_payload = match payload_kind {
        0 => SumPayload::Unit,
        1 => {
            let (count, consumed) = decode_varint(payload, pos)?;
            pos += consumed;
            let count = usize::try_from(count).map_err(|_| SurpError::LengthOverflow(count))?;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                let (item, consumed_item) = decode_segment(payload, pos, symbols, depth)?;
                pos += consumed_item;
                items.push(item);
            }
            SumPayload::Tuple(items)
        }
        2 => {
            let (count, consumed) = decode_varint(payload, pos)?;
            pos += consumed;
            let count = usize::try_from(count).map_err(|_| SurpError::LengthOverflow(count))?;
            let mut fields = Vec::with_capacity(count);
            for _ in 0..count {
                let (name_seg, consumed_name) = decode_segment(payload, pos, symbols, depth)?;
                pos += consumed_name;
                let name = expect_field_name(&name_seg)?;

                let (value, consumed_value) = decode_segment(payload, pos, symbols, depth)?;
                pos += consumed_value;

                fields.push(Field { name, value });
            }
            SumPayload::Struct(fields)
        }
        _ => {
            return Err(SurpError::InvalidData(format!(
                "unknown enum payload kind {payload_kind}"
            )));
        }
    };

    if pos != payload.len() {
        return Err(SurpError::InvalidData("enum payload trailing bytes".into()));
    }

    Ok(Value::Sum(Sum {
        type_name,
        variant,
        payload: variant_payload,
    }))
}

fn decode_sequence(payload: &[u8], symbols: &[String], depth: usize, cfg: u8) -> Result<Value> {
    let mut pos = 0usize;

    if pos >= payload.len() {
        return Err(SurpError::UnexpectedEof(pos));
    }

    let has_type = payload[pos];
    pos += 1;

    let elem_type = if has_type == 1 {
        let (value, consumed) = decode_segment(payload, pos, symbols, depth)?;
        pos += consumed;
        Some(expect_string_scalar(&value, "sequence element type")?)
    } else {
        None
    };

    let (count, consumed_count) = decode_varint(payload, pos)?;
    pos += consumed_count;
    let count = usize::try_from(count).map_err(|_| SurpError::LengthOverflow(count))?;

    let has_offsets = cfg & SEQ_HAS_OFFSET_TABLE != 0;

    let mut items = Vec::with_capacity(count);
    if has_offsets {
        let table_start = pos;
        let table_len = count
            .checked_mul(4)
            .ok_or(SurpError::LengthOverflow(count as u64))?;
        let table_end = table_start
            .checked_add(table_len)
            .ok_or(SurpError::LengthOverflow(table_len as u64))?;
        if table_end > payload.len() {
            return Err(SurpError::UnexpectedEof(table_end));
        }

        for idx in 0..count {
            let off_pos = table_start + idx * 4;
            let off = u32::from_le_bytes(
                payload[off_pos..off_pos + 4]
                    .try_into()
                    .expect("offset entry length"),
            ) as usize;
            let item_start = table_end
                .checked_add(off)
                .ok_or(SurpError::LengthOverflow(off as u64))?;
            if item_start >= payload.len() {
                return Err(SurpError::UnexpectedEof(item_start));
            }
            let (item, _consumed) = decode_segment(payload, item_start, symbols, depth)?;
            items.push(item);
        }
    } else {
        for _ in 0..count {
            let (item, consumed_item) = decode_segment(payload, pos, symbols, depth)?;
            pos += consumed_item;
            items.push(item);
        }
        if pos != payload.len() {
            return Err(SurpError::InvalidData(
                "sequence payload has trailing bytes".into(),
            ));
        }
    }

    Ok(Value::Sequence(Sequence { elem_type, items }))
}

fn decode_map(payload: &[u8], symbols: &[String], depth: usize, cfg: u8) -> Result<Value> {
    let mut pos = 0usize;
    let (count, consumed_count) = decode_varint(payload, pos)?;
    pos += consumed_count;
    let count = usize::try_from(count).map_err(|_| SurpError::LengthOverflow(count))?;

    let has_offsets = cfg & MAP_HAS_OFFSET_TABLE != 0;

    let mut pairs = Vec::with_capacity(count);
    if has_offsets {
        let table_start = pos;
        let table_len = count
            .checked_mul(4)
            .ok_or(SurpError::LengthOverflow(count as u64))?;
        let table_end = table_start
            .checked_add(table_len)
            .ok_or(SurpError::LengthOverflow(table_len as u64))?;
        if table_end > payload.len() {
            return Err(SurpError::UnexpectedEof(table_end));
        }

        for idx in 0..count {
            let off_pos = table_start + idx * 4;
            let off = u32::from_le_bytes(
                payload[off_pos..off_pos + 4]
                    .try_into()
                    .expect("map offset entry length"),
            ) as usize;
            let pair_start = table_end
                .checked_add(off)
                .ok_or(SurpError::LengthOverflow(off as u64))?;
            if pair_start >= payload.len() {
                return Err(SurpError::UnexpectedEof(pair_start));
            }

            let (key, key_consumed) = decode_segment(payload, pair_start, symbols, depth)?;
            let (value, _value_consumed) =
                decode_segment(payload, pair_start + key_consumed, symbols, depth)?;
            pairs.push((key, value));
        }
    } else {
        for _ in 0..count {
            let (key, consumed_key) = decode_segment(payload, pos, symbols, depth)?;
            pos += consumed_key;
            let (value, consumed_value) = decode_segment(payload, pos, symbols, depth)?;
            pos += consumed_value;
            pairs.push((key, value));
        }
        if pos != payload.len() {
            return Err(SurpError::InvalidData(
                "map payload has trailing bytes".into(),
            ));
        }
    }

    Ok(Value::Association(pairs))
}

fn decode_reference(payload: &[u8], symbols: &[String], depth: usize) -> Result<Value> {
    if payload.is_empty() {
        return Err(SurpError::UnexpectedEof(0));
    }

    let kind = payload[0];
    match kind {
        0 => {
            let (len, consumed) = decode_varint(payload, 1)?;
            let len = usize::try_from(len).map_err(|_| SurpError::LengthOverflow(len))?;
            let start = 1 + consumed;
            let end = start
                .checked_add(len)
                .ok_or(SurpError::LengthOverflow(len as u64))?;
            if end > payload.len() {
                return Err(SurpError::UnexpectedEof(end));
            }
            let name = std::str::from_utf8(&payload[start..end])
                .map_err(|_| SurpError::InvalidUtf8(start))?
                .to_string();
            Ok(Value::Reference(Reference::Binding(name)))
        }
        1 => {
            let (value, consumed) = decode_segment(payload, 1, symbols, depth)?;
            if 1 + consumed != payload.len() {
                return Err(SurpError::InvalidData(
                    "reference-by-id payload trailing bytes".into(),
                ));
            }
            Ok(Value::Reference(Reference::ById(Box::new(value))))
        }
        _ => Err(SurpError::InvalidData(format!(
            "unknown reference kind {kind}"
        ))),
    }
}

fn decode_tensor(payload: &[u8]) -> Result<Value> {
    if payload.len() < 4 {
        return Err(SurpError::UnexpectedEof(payload.len()));
    }

    let element_type = tensor_element_type_name(payload[0]).to_string();
    let ndim = payload[1] as usize;
    let _flags = payload[2];
    let _reserved = payload[3];

    let mut pos = 4usize;
    let mut shape = Vec::with_capacity(ndim);
    for _ in 0..ndim {
        let (dim, consumed) = decode_varint(payload, pos)?;
        pos += consumed;
        if dim == 0 {
            shape.push(None);
        } else {
            shape.push(Some(dim));
        }
    }

    if pos + 8 > payload.len() {
        return Err(SurpError::UnexpectedEof(pos));
    }
    let element_count = u64::from_le_bytes(
        payload[pos..pos + 8]
            .try_into()
            .expect("tensor element count length"),
    ) as usize;
    pos += 8;

    let data = match payload[0] {
        0x01 | 0x02 => {
            let bytes_needed = element_count
                .checked_mul(8)
                .ok_or(SurpError::LengthOverflow(element_count as u64))?;
            if pos + bytes_needed > payload.len() {
                return Err(SurpError::UnexpectedEof(pos + bytes_needed));
            }
            let mut values = Vec::with_capacity(element_count);
            let mut cursor = pos;
            for _ in 0..element_count {
                let v = f64::from_le_bytes(
                    payload[cursor..cursor + 8]
                        .try_into()
                        .expect("tensor f64 element length"),
                );
                values.push(v);
                cursor += 8;
            }
            TensorData::DenseF64(values)
        }
        0x10 => {
            let bytes_needed = element_count
                .checked_mul(8)
                .ok_or(SurpError::LengthOverflow(element_count as u64))?;
            if pos + bytes_needed > payload.len() {
                return Err(SurpError::UnexpectedEof(pos + bytes_needed));
            }
            let mut values = Vec::with_capacity(element_count);
            let mut cursor = pos;
            for _ in 0..element_count {
                let v = i64::from_le_bytes(
                    payload[cursor..cursor + 8]
                        .try_into()
                        .expect("tensor i64 element length"),
                );
                values.push(v);
                cursor += 8;
            }
            TensorData::DenseI64(values)
        }
        0x11 => {
            let bytes_needed = element_count
                .checked_mul(8)
                .ok_or(SurpError::LengthOverflow(element_count as u64))?;
            if pos + bytes_needed > payload.len() {
                return Err(SurpError::UnexpectedEof(pos + bytes_needed));
            }
            let mut values = Vec::with_capacity(element_count);
            let mut cursor = pos;
            for _ in 0..element_count {
                let v = u64::from_le_bytes(
                    payload[cursor..cursor + 8]
                        .try_into()
                        .expect("tensor u64 element length"),
                );
                values.push(v);
                cursor += 8;
            }
            TensorData::DenseU64(values)
        }
        _ => {
            let blob = payload[pos..].to_vec();
            TensorData::BinaryBlob(blob)
        }
    };

    Ok(Value::Tensor(Tensor {
        element_type,
        shape,
        data,
        annotations: Vec::new(),
    }))
}

fn decode_stream(payload: &[u8], symbols: &[String], depth: usize) -> Result<Value> {
    let mut pos = 0usize;
    let (item_type_value, consumed_type) = decode_segment(payload, pos, symbols, depth)?;
    pos += consumed_type;
    let item_type = expect_string_scalar(&item_type_value, "stream item type")?;

    let (ann_count, consumed_count) = decode_varint(payload, pos)?;
    pos += consumed_count;
    let ann_count = usize::try_from(ann_count).map_err(|_| SurpError::LengthOverflow(ann_count))?;

    let mut annotations = Vec::with_capacity(ann_count);
    for _ in 0..ann_count {
        if pos >= payload.len() {
            return Err(SurpError::UnexpectedEof(pos));
        }
        let (name_value, consumed_name) = decode_segment(payload, pos, symbols, depth)?;
        pos += consumed_name;
        let name = expect_string_scalar(&name_value, "annotation name")?;

        let has_value = payload[pos];
        pos += 1;
        let value = if has_value == 1 {
            let (scalar_value, consumed_scalar) = decode_segment(payload, pos, symbols, depth)?;
            pos += consumed_scalar;
            match scalar_value {
                Value::Scalar(s) => Some(s),
                _ => {
                    return Err(SurpError::InvalidData(
                        "stream annotation value must be scalar".into(),
                    ));
                }
            }
        } else {
            None
        };

        annotations.push(Annotation { name, value });
    }

    if pos != payload.len() {
        return Err(SurpError::InvalidData(
            "stream payload trailing bytes".into(),
        ));
    }

    Ok(Value::Stream(Stream {
        item_type,
        annotations,
    }))
}

fn decode_opaque(payload: &[u8], cfg: u8) -> Result<Value> {
    let mut pos = 0usize;
    let (tag_len, consumed_tag_len) = decode_varint(payload, pos)?;
    pos += consumed_tag_len;
    let tag_len = usize::try_from(tag_len).map_err(|_| SurpError::LengthOverflow(tag_len))?;

    let tag_end = pos
        .checked_add(tag_len)
        .ok_or(SurpError::LengthOverflow(tag_len as u64))?;
    if tag_end > payload.len() {
        return Err(SurpError::UnexpectedEof(tag_end));
    }

    let tag = std::str::from_utf8(&payload[pos..tag_end])
        .map_err(|_| SurpError::InvalidUtf8(pos))?
        .to_string();
    pos = tag_end;

    let (value_len, consumed_value_len) = decode_varint(payload, pos)?;
    pos += consumed_value_len;
    let value_len = usize::try_from(value_len).map_err(|_| SurpError::LengthOverflow(value_len))?;
    let value_end = pos
        .checked_add(value_len)
        .ok_or(SurpError::LengthOverflow(value_len as u64))?;
    if value_end > payload.len() {
        return Err(SurpError::UnexpectedEof(value_end));
    }

    let value_bytes = payload[pos..value_end].to_vec();
    pos = value_end;

    if pos != payload.len() {
        return Err(SurpError::InvalidData(
            "opaque payload trailing bytes".into(),
        ));
    }

    if cfg == 0 {
        let value = String::from_utf8(value_bytes)
            .map_err(|e| SurpError::InvalidData(format!("invalid opaque tagged utf8: {e}")))?;
        return Ok(Value::Scalar(Scalar::Tagged { tag, value }));
    }

    Ok(Value::Opaque(Opaque {
        type_tag: tag,
        bytes: value_bytes,
    }))
}

fn decode_annotation(payload: &[u8], symbols: &[String], depth: usize) -> Result<Annotation> {
    let mut pos = 0usize;
    let (name_value, consumed_name) = decode_segment(payload, pos, symbols, depth)?;
    pos += consumed_name;
    let name = expect_string_scalar(&name_value, "annotation name")?;

    if pos >= payload.len() {
        return Err(SurpError::UnexpectedEof(pos));
    }

    let has_value = payload[pos];
    pos += 1;
    let value = if has_value == 1 {
        let (value_segment, consumed_value) = decode_segment(payload, pos, symbols, depth)?;
        pos += consumed_value;
        match value_segment {
            Value::Scalar(s) => Some(s),
            _ => {
                return Err(SurpError::InvalidData(
                    "annotation value must be scalar".into(),
                ));
            }
        }
    } else {
        None
    };

    if pos != payload.len() {
        return Err(SurpError::InvalidData(
            "annotation payload trailing bytes".into(),
        ));
    }

    Ok(Annotation { name, value })
}

fn expect_string_scalar(value: &Value, context: &str) -> Result<String> {
    match value {
        Value::Scalar(Scalar::Str(s)) => Ok(s.clone()),
        Value::Scalar(Scalar::Sym(s)) => Ok(s.clone()),
        _ => Err(SurpError::InvalidData(format!(
            "expected string scalar for {context}"
        ))),
    }
}

fn expect_field_name(value: &Value) -> Result<String> {
    match value {
        Value::Scalar(Scalar::Str(s)) => Ok(s.clone()),
        Value::Scalar(Scalar::Sym(s)) => Ok(s.clone()),
        _ => Err(SurpError::InvalidData(
            "field name segment must be str or sym".into(),
        )),
    }
}

fn tensor_element_type_code(element_type: &str) -> u8 {
    match element_type {
        "f32" => 0x01,
        "f64" => 0x02,
        "i64" => 0x10,
        "u64" => 0x11,
        _ => 0xFF,
    }
}

fn tensor_element_type_name(code: u8) -> &'static str {
    match code {
        0x01 => "f32",
        0x02 => "f64",
        0x10 => "i64",
        0x11 => "u64",
        _ => "bytes",
    }
}

fn encode_symbol_table(symbols: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(symbols.len() as u32).to_le_bytes());
    for sym in symbols {
        encode_varint_vec(sym.len() as u64, &mut out);
        out.extend_from_slice(sym.as_bytes());
    }
    out
}

fn decode_symbol_table(payload: &[u8], offset: usize) -> Result<Vec<String>> {
    if offset + 4 > payload.len() {
        return Err(SurpError::UnexpectedEof(offset));
    }

    let count = u32::from_le_bytes(
        payload[offset..offset + 4]
            .try_into()
            .expect("symbol table count bytes"),
    ) as usize;

    let mut pos = offset + 4;
    let mut symbols = Vec::with_capacity(count);
    for _ in 0..count {
        let (len, consumed) = decode_varint(payload, pos)?;
        pos += consumed;
        let len = usize::try_from(len).map_err(|_| SurpError::LengthOverflow(len))?;
        let end = pos
            .checked_add(len)
            .ok_or(SurpError::LengthOverflow(len as u64))?;
        if end > payload.len() {
            return Err(SurpError::UnexpectedEof(end));
        }
        let sym = std::str::from_utf8(&payload[pos..end])
            .map_err(|_| SurpError::InvalidUtf8(pos))?
            .to_string();
        symbols.push(sym);
        pos = end;
    }

    Ok(symbols)
}

fn collect_symbols(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Scalar(Scalar::Sym(sym)) => {
            out.insert(sym.clone());
        }
        Value::Scalar(_) => {}
        Value::Product(product) => {
            for field in &product.fields {
                out.insert(field.name.clone());
                collect_symbols(&field.value, out);
            }
            if let Some(name) = &product.type_name {
                out.insert(name.clone());
            }
        }
        Value::Sum(sum) => {
            out.insert(sum.variant.clone());
            if let Some(name) = &sum.type_name {
                out.insert(name.clone());
            }
            match &sum.payload {
                SumPayload::Unit => {}
                SumPayload::Tuple(items) => {
                    for item in items {
                        collect_symbols(item, out);
                    }
                }
                SumPayload::Struct(fields) => {
                    for field in fields {
                        out.insert(field.name.clone());
                        collect_symbols(&field.value, out);
                    }
                }
            }
        }
        Value::Sequence(seq) => {
            if let Some(elem_type) = &seq.elem_type {
                out.insert(elem_type.clone());
            }
            for item in &seq.items {
                collect_symbols(item, out);
            }
        }
        Value::Association(map) => {
            for (k, v) in map {
                collect_symbols(k, out);
                collect_symbols(v, out);
            }
        }
        Value::Reference(Reference::Binding(name)) => {
            out.insert(name.clone());
        }
        Value::Reference(Reference::ById(id)) => collect_symbols(id, out),
        Value::Tensor(tensor) => {
            out.insert(tensor.element_type.clone());
            for ann in &tensor.annotations {
                out.insert(ann.name.clone());
                if let Some(Scalar::Sym(sym)) = &ann.value {
                    out.insert(sym.clone());
                }
            }
        }
        Value::Stream(stream) => {
            out.insert(stream.item_type.clone());
            for ann in &stream.annotations {
                out.insert(ann.name.clone());
                if let Some(Scalar::Sym(sym)) = &ann.value {
                    out.insert(sym.clone());
                }
            }
        }
        Value::Opaque(opaque) => {
            out.insert(opaque.type_tag.clone());
        }
    }
}

fn contains_symbol_value(value: &Value) -> bool {
    match value {
        Value::Scalar(Scalar::Sym(_)) => true,
        Value::Scalar(_) => false,
        Value::Product(product) => product
            .fields
            .iter()
            .any(|f| contains_symbol_value(&f.value)),
        Value::Sum(sum) => match &sum.payload {
            SumPayload::Unit => false,
            SumPayload::Tuple(items) => items.iter().any(contains_symbol_value),
            SumPayload::Struct(fields) => fields.iter().any(|f| contains_symbol_value(&f.value)),
        },
        Value::Sequence(seq) => seq.items.iter().any(contains_symbol_value),
        Value::Association(map) => map
            .iter()
            .any(|(k, v)| contains_symbol_value(k) || contains_symbol_value(v)),
        Value::Reference(Reference::Binding(_)) => false,
        Value::Reference(Reference::ById(v)) => contains_symbol_value(v),
        Value::Tensor(tensor) => tensor
            .annotations
            .iter()
            .any(|ann| matches!(ann.value, Some(Scalar::Sym(_)))),
        Value::Stream(stream) => stream
            .annotations
            .iter()
            .any(|ann| matches!(ann.value, Some(Scalar::Sym(_)))),
        Value::Opaque(_) => false,
    }
}

fn resolve_references(value: &Value, doc: &Document, stack: &mut HashSet<String>) -> Result<Value> {
    match value {
        Value::Reference(Reference::Binding(name)) => {
            if let Some(binding) = doc.binding(name) {
                if !stack.insert(name.clone()) {
                    return Err(SurpError::InvalidData(format!(
                        "cyclic reference detected at binding '{name}'"
                    )));
                }
                let resolved = resolve_references(&binding.value, doc, stack)?;
                stack.remove(name);
                Ok(resolved)
            } else {
                Ok(value.clone())
            }
        }
        Value::Reference(Reference::ById(id)) => Ok(Value::Reference(Reference::ById(Box::new(
            resolve_references(id, doc, stack)?,
        )))),
        Value::Product(product) => {
            let mut fields = Vec::with_capacity(product.fields.len());
            for field in &product.fields {
                fields.push(Field {
                    name: field.name.clone(),
                    value: resolve_references(&field.value, doc, stack)?,
                });
            }
            Ok(Value::Product(Product {
                type_name: product.type_name.clone(),
                fields,
            }))
        }
        Value::Sum(sum) => {
            let payload = match &sum.payload {
                SumPayload::Unit => SumPayload::Unit,
                SumPayload::Tuple(items) => {
                    let mut resolved = Vec::with_capacity(items.len());
                    for item in items {
                        resolved.push(resolve_references(item, doc, stack)?);
                    }
                    SumPayload::Tuple(resolved)
                }
                SumPayload::Struct(fields) => {
                    let mut resolved = Vec::with_capacity(fields.len());
                    for field in fields {
                        resolved.push(Field {
                            name: field.name.clone(),
                            value: resolve_references(&field.value, doc, stack)?,
                        });
                    }
                    SumPayload::Struct(resolved)
                }
            };
            Ok(Value::Sum(Sum {
                type_name: sum.type_name.clone(),
                variant: sum.variant.clone(),
                payload,
            }))
        }
        Value::Sequence(seq) => {
            let mut items = Vec::with_capacity(seq.items.len());
            for item in &seq.items {
                items.push(resolve_references(item, doc, stack)?);
            }
            Ok(Value::Sequence(Sequence {
                elem_type: seq.elem_type.clone(),
                items,
            }))
        }
        Value::Association(map) => {
            let mut pairs = Vec::with_capacity(map.len());
            for (k, v) in map {
                pairs.push((
                    resolve_references(k, doc, stack)?,
                    resolve_references(v, doc, stack)?,
                ));
            }
            Ok(Value::Association(pairs))
        }
        Value::Tensor(tensor) => Ok(Value::Tensor(tensor.clone())),
        Value::Stream(stream) => Ok(Value::Stream(stream.clone())),
        Value::Opaque(opaque) => Ok(Value::Opaque(opaque.clone())),
        Value::Scalar(_) => Ok(value.clone()),
    }
}

fn crc64_ecma(data: &[u8]) -> u64 {
    const POLY: u64 = 0x42F0_E1EB_A9EA_3693;
    let mut crc = 0u64;

    for &byte in data {
        crc ^= (byte as u64) << 56;
        for _ in 0..8 {
            if (crc & 0x8000_0000_0000_0000) != 0 {
                crc = (crc << 1) ^ POLY;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rfc001::ast::{Binding, Scalar, Value};

    #[test]
    fn header_roundtrip() {
        let mut header = CbfHeader::new();
        header.flags |= FLAG_HAS_SYMTAB;
        header.root_offset = 42;
        header.symtab_offset = 0;
        header.alignment = 6;

        let bytes = header.encode();
        let decoded = CbfHeader::decode(&bytes).unwrap();
        assert_eq!(decoded, header);
    }

    #[test]
    fn scalar_roundtrip() {
        let value = Value::Scalar(Scalar::Vi64(-42));
        let encoded = encode_value(&value, EncodeOptions::default()).unwrap();
        let decoded = decode_value(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn product_roundtrip_with_symbols() {
        let value = Value::Product(Product {
            type_name: Some("User".into()),
            fields: vec![
                Field {
                    name: "id".into(),
                    value: Value::Scalar(Scalar::Tagged {
                        tag: "uid".into(),
                        value: "550e8400-e29b-41d4-a716-446655440000".into(),
                    }),
                },
                Field {
                    name: "role".into(),
                    value: Value::Scalar(Scalar::Sym("Admin".into())),
                },
            ],
        });

        let encoded = encode_value(&value, EncodeOptions::default()).unwrap();
        let decoded = decode_value(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn sequence_map_tensor_roundtrip() {
        let value = Value::Association(vec![
            (
                Value::Scalar(Scalar::Str("vectors".into())),
                Value::Sequence(Sequence {
                    elem_type: Some("tensor<f64>".into()),
                    items: vec![Value::Tensor(Tensor {
                        element_type: "f64".into(),
                        shape: vec![Some(2)],
                        data: TensorData::DenseF64(vec![1.0, 2.0]),
                        annotations: Vec::new(),
                    })],
                }),
            ),
            (
                Value::Scalar(Scalar::Str("active".into())),
                Value::Scalar(Scalar::Bool(true)),
            ),
        ]);

        let encoded = encode_value(&value, EncodeOptions::default()).unwrap();
        let decoded = decode_value(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn crc_detects_corruption() {
        let value = Value::Scalar(Scalar::Str("hello".into()));
        let mut encoded = encode_value(&value, EncodeOptions::default()).unwrap();
        let idx = CBF_HEADER_SIZE + 2;
        encoded[idx] ^= 0xFF;
        let err = decode_value(&encoded).unwrap_err();
        assert!(matches!(err, SurpError::ChecksumMismatch { .. }));
    }

    #[test]
    fn resolves_references_before_encoding() {
        let doc = Document {
            bindings: vec![Binding {
                name: "alice".into(),
                value: Value::Product(Product {
                    type_name: Some("User".into()),
                    fields: vec![Field {
                        name: "name".into(),
                        value: Value::Scalar(Scalar::Str("Alice".into())),
                    }],
                }),
            }],
            root: Some(Value::Reference(Reference::Binding("alice".into()))),
            ..Document::default()
        };

        let encoded = encode_document(&doc, EncodeOptions::default()).unwrap();
        let decoded = decode_document(&encoded).unwrap();
        let root = decoded.document.effective_root().unwrap();
        assert!(matches!(root, Value::Product(_)));
    }
}
