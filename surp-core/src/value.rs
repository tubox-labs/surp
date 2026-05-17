//! Value types for schema-less Surp data.
//!
//! Two representations:
//! - `Value`: owned, heap-allocated, suitable for building documents.
//! - `SurpValue<'a>`: zero-copy, borrow-backed, returned by the decoder.

use std::fmt;

/// Owned, schema-less Surp value (analogous to `serde_json::Value`).
///
/// This type owns all its data and can be freely moved, cloned, and serialized.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Null / absent value.
    Null,
    /// Boolean.
    Bool(bool),
    /// Unsigned integer (up to u64).
    UInt(u64),
    /// Signed integer (i64).
    Int(i64),
    /// 64-bit IEEE 754 float.
    Float(f64),
    /// UTF-8 string.
    Str(String),
    /// Raw binary blob.
    Bytes(Vec<u8>),
    /// Ordered array of values.
    Array(Vec<Value>),
    /// Ordered map of key-value pairs (keys are strings).
    /// Uses Vec to preserve insertion order (deterministic encoding).
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Returns the type name as a human-readable string.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::UInt(_) => "uint",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::Bytes(_) => "bytes",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Returns true if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Try to get as a string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as u64.
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Value::UInt(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as i64.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as f64.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as an array.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Try to get as an object (ordered key-value pairs).
    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Try to get as a raw byte slice.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Return the logical item count for arrays and objects.
    ///
    /// Scalars report `0`, mirroring JSON-style container introspection without
    /// changing scalar semantics.
    pub fn len(&self) -> usize {
        match self {
            Value::Array(items) => items.len(),
            Value::Object(entries) => entries.len(),
            _ => 0,
        }
    }

    /// Return true when an array or object has no entries, or when this is a scalar.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return true if this value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Return true if this value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    /// Return true if this value is a scalar value.
    pub fn is_scalar(&self) -> bool {
        !matches!(self, Value::Array(_) | Value::Object(_))
    }

    /// Look up an object field by key, preserving the first matching entry.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Return true if this object contains `key`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Look up an array item by index.
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(items) => items.get(index),
            _ => None,
        }
    }

    /// Return object keys in encoded/insertion order.
    pub fn keys(&self) -> Vec<&str> {
        match self {
            Value::Object(entries) => entries.iter().map(|(k, _)| k.as_str()).collect(),
            _ => Vec::new(),
        }
    }

    /// Return array items or object values in encoded/insertion order.
    pub fn values(&self) -> Vec<&Value> {
        match self {
            Value::Array(items) => items.iter().collect(),
            Value::Object(entries) => entries.iter().map(|(_, v)| v).collect(),
            _ => Vec::new(),
        }
    }

    /// Return object entries in encoded/insertion order.
    pub fn entries(&self) -> Option<&[(String, Value)]> {
        self.as_object()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::UInt(n) => write!(f, "{n}"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Bytes(b) => write!(
                f,
                "b64#{}",
                base64::engine::general_purpose::STANDARD.encode(b)
            ),
            Value::Array(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Value::Object(entries) => {
                write!(f, "{{ ")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{k}: {v};")?;
                }
                write!(f, " }}")
            }
        }
    }
}

use base64::Engine;

/// Zero-copy, borrow-backed Surp value returned by the decoder.
///
/// Borrows string and bytes data directly from the input buffer,
/// avoiding allocation for read-heavy workloads.
#[derive(Debug, Clone, PartialEq)]
pub enum SurpValue<'a> {
    Null,
    Bool(bool),
    UInt(u64),
    Int(i64),
    Float(f64),
    /// Borrowed UTF-8 string slice from the input buffer.
    Str(&'a str),
    /// Borrowed byte slice from the input buffer.
    Bytes(&'a [u8]),
    /// Array of zero-copy values.
    Array(Vec<SurpValue<'a>>),
    /// Object with borrowed keys.
    Object(Vec<(&'a str, SurpValue<'a>)>),
}

impl<'a> SurpValue<'a> {
    /// Convert a borrowed SurpValue into an owned Value (copies strings/bytes).
    pub fn to_owned_value(&self) -> Value {
        match self {
            SurpValue::Null => Value::Null,
            SurpValue::Bool(b) => Value::Bool(*b),
            SurpValue::UInt(n) => Value::UInt(*n),
            SurpValue::Int(n) => Value::Int(*n),
            SurpValue::Float(n) => Value::Float(*n),
            SurpValue::Str(s) => Value::Str((*s).to_string()),
            SurpValue::Bytes(b) => Value::Bytes(b.to_vec()),
            SurpValue::Array(items) => {
                Value::Array(items.iter().map(|v| v.to_owned_value()).collect())
            }
            SurpValue::Object(entries) => Value::Object(
                entries
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), v.to_owned_value()))
                    .collect(),
            ),
        }
    }

    /// Returns the type name as a human-readable string.
    pub fn type_name(&self) -> &'static str {
        match self {
            SurpValue::Null => "null",
            SurpValue::Bool(_) => "bool",
            SurpValue::UInt(_) => "uint",
            SurpValue::Int(_) => "int",
            SurpValue::Float(_) => "float",
            SurpValue::Str(_) => "str",
            SurpValue::Bytes(_) => "bytes",
            SurpValue::Array(_) => "array",
            SurpValue::Object(_) => "object",
        }
    }

    /// Returns true if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, SurpValue::Null)
    }

    /// Return true if this value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, SurpValue::Array(_))
    }

    /// Return true if this value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, SurpValue::Object(_))
    }

    /// Return true if this value is a scalar value.
    pub fn is_scalar(&self) -> bool {
        !matches!(self, SurpValue::Array(_) | SurpValue::Object(_))
    }

    /// Try to get as a string slice.
    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            SurpValue::Str(s) => Some(*s),
            _ => None,
        }
    }

    /// Try to get as a raw byte slice.
    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match self {
            SurpValue::Bytes(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as u64.
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            SurpValue::UInt(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as i64.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            SurpValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as f64.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            SurpValue::Float(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SurpValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Return the logical item count for arrays and objects.
    pub fn len(&self) -> usize {
        match self {
            SurpValue::Array(items) => items.len(),
            SurpValue::Object(entries) => entries.len(),
            _ => 0,
        }
    }

    /// Return true when an array or object has no entries, or when this is a scalar.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Look up an object field by key, preserving the first matching entry.
    pub fn get(&self, key: &str) -> Option<&SurpValue<'a>> {
        match self {
            SurpValue::Object(entries) => entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Return true if this object contains `key`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Look up an array item by index.
    pub fn get_index(&self, index: usize) -> Option<&SurpValue<'a>> {
        match self {
            SurpValue::Array(items) => items.get(index),
            _ => None,
        }
    }

    /// Return object keys in encoded/insertion order.
    pub fn keys(&self) -> Vec<&'a str> {
        match self {
            SurpValue::Object(entries) => entries.iter().map(|(k, _)| *k).collect(),
            _ => Vec::new(),
        }
    }

    /// Return array items or object values in encoded/insertion order.
    pub fn values(&self) -> Vec<&SurpValue<'a>> {
        match self {
            SurpValue::Array(items) => items.iter().collect(),
            SurpValue::Object(entries) => entries.iter().map(|(_, v)| v).collect(),
            _ => Vec::new(),
        }
    }

    /// Try to get as an array.
    pub fn as_array(&self) -> Option<&[SurpValue<'a>]> {
        match self {
            SurpValue::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Try to get as an object (ordered key-value pairs).
    pub fn as_object(&self) -> Option<&[(&'a str, SurpValue<'a>)]> {
        match self {
            SurpValue::Object(entries) => Some(entries),
            _ => None,
        }
    }
}

/// Convert from serde_json::Value to surp Value for interop.
impl From<&serde_json::Value> for Value {
    fn from(jv: &serde_json::Value) -> Self {
        match jv {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    Value::UInt(u)
                } else if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::Str(s.clone()),
            serde_json::Value::Array(arr) => Value::Array(arr.iter().map(Value::from).collect()),
            serde_json::Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), Value::from(v)))
                    .collect(),
            ),
        }
    }
}

/// Convert from surp Value to serde_json::Value for interop.
impl From<&Value> for serde_json::Value {
    fn from(cv: &Value) -> Self {
        match cv {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::UInt(n) => serde_json::json!(*n),
            Value::Int(n) => serde_json::json!(*n),
            Value::Float(n) => serde_json::json!(*n),
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::Bytes(b) => {
                serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
            }
            Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(serde_json::Value::from).collect())
            }
            Value::Object(entries) => {
                let map: serde_json::Map<String, serde_json::Value> = entries
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::from(v)))
                    .collect();
                serde_json::Value::Object(map)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_type_names() {
        assert_eq!(Value::Null.type_name(), "null");
        assert_eq!(Value::Bool(true).type_name(), "bool");
        assert_eq!(Value::UInt(42).type_name(), "uint");
        assert_eq!(Value::Str("hi".into()).type_name(), "str");
    }

    #[test]
    fn surp_value_to_owned() {
        let cv = SurpValue::Object(vec![
            ("name", SurpValue::Str("Alice")),
            ("age", SurpValue::UInt(30)),
        ]);
        let owned = cv.to_owned_value();
        assert_eq!(
            owned,
            Value::Object(vec![
                ("name".into(), Value::Str("Alice".into())),
                ("age".into(), Value::UInt(30)),
            ])
        );
    }

    #[test]
    fn json_roundtrip() {
        let cv = Value::Object(vec![
            ("name".into(), Value::Str("Bob".into())),
            ("score".into(), Value::Float(99.5)),
        ]);
        let jv = serde_json::Value::from(&cv);
        let back = Value::from(&jv);
        // Float comes back as Float since serde_json preserves f64
        match &back {
            Value::Object(entries) => {
                assert_eq!(entries[0].0, "name");
                assert_eq!(entries[1].0, "score");
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn value_introspection_helpers() {
        let value = Value::Object(vec![
            ("name".into(), Value::Str("Alice".into())),
            (
                "tags".into(),
                Value::Array(vec![Value::Str("admin".into()), Value::Str("ops".into())]),
            ),
        ]);

        assert!(value.is_object());
        assert_eq!(value.len(), 2);
        assert_eq!(value.keys(), vec!["name", "tags"]);
        assert_eq!(value.get("name").and_then(Value::as_str), Some("Alice"));
        assert!(value.contains_key("tags"));

        let tags = value.get("tags").unwrap();
        assert!(tags.is_array());
        assert_eq!(tags.len(), 2);
        assert_eq!(tags.get_index(1).and_then(Value::as_str), Some("ops"));
    }

    #[test]
    fn surp_value_introspection_helpers() {
        let value = SurpValue::Object(vec![
            ("name", SurpValue::Str("Alice")),
            (
                "tags",
                SurpValue::Array(vec![SurpValue::Str("admin"), SurpValue::Str("ops")]),
            ),
        ]);

        assert_eq!(value.type_name(), "object");
        assert_eq!(value.keys(), vec!["name", "tags"]);
        assert_eq!(value.get("name").and_then(SurpValue::as_str), Some("Alice"));
        assert_eq!(
            value
                .get("tags")
                .and_then(|tags| tags.get_index(1))
                .and_then(SurpValue::as_str),
            Some("ops")
        );
    }
}
