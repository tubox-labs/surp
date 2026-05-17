//! RFC-001 implementation modules.
//!
//! This namespace introduces the next-generation Crous architecture in
//! parallel to the stable v1 APIs.

pub mod ast;
pub mod cbf;
pub mod cql;
pub mod ctn;

pub use ast::{
    Annotation, Binding, Document, Field, Opaque, Product, Reference, Scalar, Sequence, Stream,
    Sum, SumPayload, Tensor, TensorData, Value,
};
pub use cbf::{
    CBF_HEADER_SIZE, CBF_MAGIC, CbfHeader, DecodedDocument, EncodeOptions, decode_document,
    decode_value, encode_document, encode_value,
};
pub use cql::{query, query_one};
pub use ctn::{format_document, format_value, parse_document, parse_value};
