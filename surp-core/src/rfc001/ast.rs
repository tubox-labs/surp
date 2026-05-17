//! RFC-001 native data model AST.
//!
//! This module defines the in-memory representation used by the RFC-001
//! implementation (CTN + CBF + CQL). It intentionally does not reuse
//! `crate::value::Value` so the v1 API remains stable.

use crate::error::{Result, SurpError};

/// A full CTN document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    pub annotations: Vec<Annotation>,
    pub uses: Vec<String>,
    pub bindings: Vec<Binding>,
    pub root: Option<Value>,
}

impl Document {
    /// Resolve the effective root value.
    ///
    /// Priority:
    /// 1. Explicit `root` statement.
    /// 2. Last `let` binding value.
    pub fn effective_root(&self) -> Result<Value> {
        if let Some(value) = &self.root {
            return Ok(value.clone());
        }
        if let Some(binding) = self.bindings.last() {
            return Ok(binding.value.clone());
        }
        Err(SurpError::InvalidData(
            "document has no root value or bindings".into(),
        ))
    }

    /// Look up a `let` binding by name.
    pub fn binding(&self, name: &str) -> Option<&Binding> {
        self.bindings.iter().find(|b| b.name == name)
    }

    /// Look up a binding value by name.
    pub fn binding_value(&self, name: &str) -> Option<&Value> {
        self.binding(name).map(|binding| &binding.value)
    }

    /// Return binding names in document order.
    pub fn binding_names(&self) -> Vec<&str> {
        self.bindings
            .iter()
            .map(|binding| binding.name.as_str())
            .collect()
    }

    /// Look up a document annotation by name.
    pub fn annotation(&self, name: &str) -> Option<&Annotation> {
        self.annotations.iter().find(|ann| ann.name == name)
    }

    /// Return annotation names in document order.
    pub fn annotation_names(&self) -> Vec<&str> {
        self.annotations
            .iter()
            .map(|ann| ann.name.as_str())
            .collect()
    }

    /// Return true when the document contains no metadata, bindings, or root.
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
            && self.uses.is_empty()
            && self.bindings.is_empty()
            && self.root.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub name: String,
    pub value: Option<Scalar>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Scalar(Scalar),
    Product(Product),
    Sum(Sum),
    Sequence(Sequence),
    Association(Vec<(Value, Value)>),
    Reference(Reference),
    Tensor(Tensor),
    Stream(Stream),
    Opaque(Opaque),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Scalar(_) => "scalar",
            Value::Product(_) => "product",
            Value::Sum(_) => "sum",
            Value::Sequence(_) => "sequence",
            Value::Association(_) => "association",
            Value::Reference(_) => "reference",
            Value::Tensor(_) => "tensor",
            Value::Stream(_) => "stream",
            Value::Opaque(_) => "opaque",
        }
    }

    pub fn as_scalar(&self) -> Option<&Scalar> {
        match self {
            Value::Scalar(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_product(&self) -> Option<&Product> {
        match self {
            Value::Product(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Value::Sequence(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_sum(&self) -> Option<&Sum> {
        match self {
            Value::Sum(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_association(&self) -> Option<&[(Value, Value)]> {
        match self {
            Value::Association(pairs) => Some(pairs),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<&Reference> {
        match self {
            Value::Reference(reference) => Some(reference),
            _ => None,
        }
    }

    pub fn as_tensor(&self) -> Option<&Tensor> {
        match self {
            Value::Tensor(tensor) => Some(tensor),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<&Stream> {
        match self {
            Value::Stream(stream) => Some(stream),
            _ => None,
        }
    }

    pub fn as_opaque(&self) -> Option<&Opaque> {
        match self {
            Value::Opaque(opaque) => Some(opaque),
            _ => None,
        }
    }

    /// Return true if this value is a scalar.
    pub fn is_scalar(&self) -> bool {
        matches!(self, Value::Scalar(_))
    }

    /// Return the logical item count for containers and structured payloads.
    pub fn len(&self) -> usize {
        match self {
            Value::Product(product) => product.fields.len(),
            Value::Sequence(sequence) => sequence.items.len(),
            Value::Association(pairs) => pairs.len(),
            Value::Sum(sum) => sum.len(),
            Value::Tensor(tensor) => tensor.len(),
            Value::Stream(stream) => stream.annotations.len(),
            _ => 0,
        }
    }

    /// Return true when this value has no child entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Look up a product field, association key, or struct-style sum payload field.
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Product(product) => product.field(name),
            Value::Association(pairs) => pairs
                .iter()
                .find(|(key, _)| key.matches_key(name))
                .map(|(_, value)| value),
            Value::Sum(sum) => sum.field(name),
            Value::Reference(Reference::ById(inner)) => inner.get(name),
            _ => None,
        }
    }

    /// Return true if a product, association, or struct-style sum has `name`.
    pub fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Look up a sequence item or tuple-style sum payload by index.
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Sequence(sequence) => sequence.get(index),
            Value::Sum(sum) => sum.get_index(index),
            Value::Reference(Reference::ById(inner)) => inner.get_index(index),
            _ => None,
        }
    }

    /// Return field names or association keys that can be represented as strings.
    pub fn keys(&self) -> Vec<&str> {
        match self {
            Value::Product(product) => product.field_names(),
            Value::Association(pairs) => {
                pairs.iter().filter_map(|(key, _)| key.key_name()).collect()
            }
            Value::Sum(sum) => sum.field_names(),
            Value::Reference(Reference::ById(inner)) => inner.keys(),
            _ => Vec::new(),
        }
    }

    /// Return child values in structural order.
    pub fn values(&self) -> Vec<&Value> {
        match self {
            Value::Product(product) => product.fields.iter().map(|field| &field.value).collect(),
            Value::Sequence(sequence) => sequence.items.iter().collect(),
            Value::Association(pairs) => pairs.iter().map(|(_, value)| value).collect(),
            Value::Sum(sum) => sum.values(),
            Value::Reference(Reference::ById(inner)) => inner.values(),
            _ => Vec::new(),
        }
    }

    fn key_name(&self) -> Option<&str> {
        match self {
            Value::Scalar(Scalar::Str(s)) | Value::Scalar(Scalar::Sym(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    fn matches_key(&self, expected: &str) -> bool {
        self.key_name().is_some_and(|key| key == expected)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Null,
    Unit,
    Bool(bool),
    I64(i64),
    U64(u64),
    Vi64(i64),
    Vu64(u64),
    F32(f32),
    F64(f64),
    Str(String),
    Bytes(Vec<u8>),
    Sym(String),
    Tagged { tag: String, value: String },
}

impl Scalar {
    pub fn type_name(&self) -> &'static str {
        match self {
            Scalar::Null => "null",
            Scalar::Unit => "unit",
            Scalar::Bool(_) => "bool",
            Scalar::I64(_) => "i64",
            Scalar::U64(_) => "u64",
            Scalar::Vi64(_) => "vi64",
            Scalar::Vu64(_) => "vu64",
            Scalar::F32(_) => "f32",
            Scalar::F64(_) => "f64",
            Scalar::Str(_) => "str",
            Scalar::Bytes(_) => "bytes",
            Scalar::Sym(_) => "sym",
            Scalar::Tagged { .. } => "tagged",
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Scalar::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Scalar::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Scalar::I64(v) | Scalar::Vi64(v) => Some(*v),
            Scalar::U64(v) | Scalar::Vu64(v) => i64::try_from(*v).ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Scalar::F64(v) => Some(*v),
            Scalar::F32(v) => Some(f64::from(*v)),
            Scalar::I64(v) | Scalar::Vi64(v) => Some(*v as f64),
            Scalar::U64(v) | Scalar::Vu64(v) => Some(*v as f64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    pub type_name: Option<String>,
    pub fields: Vec<Field>,
}

impl Product {
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| &f.value)
    }

    pub fn contains_field(&self, name: &str) -> bool {
        self.field(name).is_some()
    }

    pub fn field_names(&self) -> Vec<&str> {
        self.fields
            .iter()
            .map(|field| field.name.as_str())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sum {
    pub type_name: Option<String>,
    pub variant: String,
    pub payload: SumPayload,
}

impl Sum {
    pub fn payload_kind(&self) -> &'static str {
        self.payload.kind()
    }

    pub fn len(&self) -> usize {
        self.payload.len()
    }

    pub fn is_empty(&self) -> bool {
        self.payload.is_empty()
    }

    pub fn field(&self, name: &str) -> Option<&Value> {
        match &self.payload {
            SumPayload::Struct(fields) => fields
                .iter()
                .find(|field| field.name == name)
                .map(|field| &field.value),
            _ => None,
        }
    }

    pub fn get_index(&self, index: usize) -> Option<&Value> {
        match &self.payload {
            SumPayload::Tuple(items) => items.get(index),
            _ => None,
        }
    }

    pub fn field_names(&self) -> Vec<&str> {
        match &self.payload {
            SumPayload::Struct(fields) => fields.iter().map(|field| field.name.as_str()).collect(),
            _ => Vec::new(),
        }
    }

    pub fn values(&self) -> Vec<&Value> {
        match &self.payload {
            SumPayload::Unit => Vec::new(),
            SumPayload::Tuple(items) => items.iter().collect(),
            SumPayload::Struct(fields) => fields.iter().map(|field| &field.value).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SumPayload {
    Unit,
    Tuple(Vec<Value>),
    Struct(Vec<Field>),
}

impl SumPayload {
    pub fn kind(&self) -> &'static str {
        match self {
            SumPayload::Unit => "unit",
            SumPayload::Tuple(_) => "tuple",
            SumPayload::Struct(_) => "struct",
        }
    }

    pub fn len(&self) -> usize {
        match self {
            SumPayload::Unit => 0,
            SumPayload::Tuple(items) => items.len(),
            SumPayload::Struct(fields) => fields.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    pub elem_type: Option<String>,
    pub items: Vec<Value>,
}

impl Sequence {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.items.get(index)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reference {
    Binding(String),
    ById(Box<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    pub element_type: String,
    pub shape: Vec<Option<u64>>,
    pub data: TensorData,
    pub annotations: Vec<Annotation>,
}

impl Tensor {
    pub fn data_kind(&self) -> &'static str {
        self.data.kind()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn annotation(&self, name: &str) -> Option<&Annotation> {
        self.annotations.iter().find(|ann| ann.name == name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TensorData {
    DenseF64(Vec<f64>),
    DenseI64(Vec<i64>),
    DenseU64(Vec<u64>),
    BinaryBlob(Vec<u8>),
}

impl TensorData {
    pub fn kind(&self) -> &'static str {
        match self {
            TensorData::DenseF64(_) => "dense_f64",
            TensorData::DenseI64(_) => "dense_i64",
            TensorData::DenseU64(_) => "dense_u64",
            TensorData::BinaryBlob(_) => "binary_blob",
        }
    }

    pub fn len(&self) -> usize {
        match self {
            TensorData::DenseF64(values) => values.len(),
            TensorData::DenseI64(values) => values.len(),
            TensorData::DenseU64(values) => values.len(),
            TensorData::BinaryBlob(bytes) => bytes.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub item_type: String,
    pub annotations: Vec<Annotation>,
}

impl Stream {
    pub fn annotation(&self, name: &str) -> Option<&Annotation> {
        self.annotations.iter().find(|ann| ann.name == name)
    }

    pub fn annotation_names(&self) -> Vec<&str> {
        self.annotations
            .iter()
            .map(|ann| ann.name.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Opaque {
    pub type_tag: String,
    pub bytes: Vec<u8>,
}

impl From<Scalar> for Value {
    fn from(value: Scalar) -> Self {
        Value::Scalar(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_root_prefers_explicit_root() {
        let doc = Document {
            bindings: vec![Binding {
                name: "a".into(),
                value: Value::Scalar(Scalar::U64(1)),
            }],
            root: Some(Value::Scalar(Scalar::U64(2))),
            ..Document::default()
        };

        assert_eq!(doc.effective_root().unwrap(), Value::Scalar(Scalar::U64(2)));
    }

    #[test]
    fn effective_root_falls_back_to_last_binding() {
        let doc = Document {
            bindings: vec![
                Binding {
                    name: "a".into(),
                    value: Value::Scalar(Scalar::U64(1)),
                },
                Binding {
                    name: "b".into(),
                    value: Value::Scalar(Scalar::U64(2)),
                },
            ],
            ..Document::default()
        };

        assert_eq!(doc.effective_root().unwrap(), Value::Scalar(Scalar::U64(2)));
    }

    #[test]
    fn scalar_numeric_views() {
        assert_eq!(Scalar::I64(-2).as_i64(), Some(-2));
        assert_eq!(Scalar::Vu64(7).as_i64(), Some(7));
        assert_eq!(Scalar::F32(1.5).as_f64(), Some(1.5));
    }

    #[test]
    fn document_introspection_helpers() {
        let doc = Document {
            annotations: vec![Annotation {
                name: "surp".into(),
                value: Some(Scalar::Str("v1".into())),
            }],
            bindings: vec![Binding {
                name: "alice".into(),
                value: Value::Scalar(Scalar::Str("Alice".into())),
            }],
            ..Document::default()
        };

        assert_eq!(doc.annotation_names(), vec!["surp"]);
        assert_eq!(doc.binding_names(), vec!["alice"]);
        assert_eq!(
            doc.binding_value("alice").and_then(Value::as_scalar),
            Some(&Scalar::Str("Alice".into()))
        );
        assert!(!doc.is_empty());
    }

    #[test]
    fn rfc_value_introspection_helpers() {
        let value = Value::Product(Product {
            type_name: Some("User".into()),
            fields: vec![
                Field {
                    name: "name".into(),
                    value: Value::Scalar(Scalar::Str("Alice".into())),
                },
                Field {
                    name: "tags".into(),
                    value: Value::Sequence(Sequence {
                        elem_type: Some("str".into()),
                        items: vec![
                            Value::Scalar(Scalar::Str("admin".into())),
                            Value::Scalar(Scalar::Str("ops".into())),
                        ],
                    }),
                },
            ],
        });

        assert_eq!(value.type_name(), "product");
        assert_eq!(value.keys(), vec!["name", "tags"]);
        assert_eq!(value.len(), 2);
        assert!(value.contains_key("tags"));
        assert_eq!(
            value
                .get("tags")
                .and_then(|tags| tags.get_index(1))
                .and_then(Value::as_scalar),
            Some(&Scalar::Str("ops".into()))
        );
    }
}
