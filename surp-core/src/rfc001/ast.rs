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

#[derive(Debug, Clone, PartialEq)]
pub enum SumPayload {
    Unit,
    Tuple(Vec<Value>),
    Struct(Vec<Field>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    pub elem_type: Option<String>,
    pub items: Vec<Value>,
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

#[derive(Debug, Clone, PartialEq)]
pub enum TensorData {
    DenseF64(Vec<f64>),
    DenseI64(Vec<i64>),
    DenseU64(Vec<u64>),
    BinaryBlob(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub item_type: String,
    pub annotations: Vec<Annotation>,
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
}
