//! # surp-python
//!
//! PyO3 native extension that wraps `surp-core` encoder/decoder for Python.
//!
//! Provides:
//! - `encode(obj)` / `decode(data)` - Simple encode/decode functions
//! - `dumps(obj, ...)` / `loads(data, ...)` - JSON-like API with options
//! - `dump(obj, fp, ...)` / `load(fp, ...)` - File-based JSON-like API
//! - `Encoder` class: Incremental encoder with dedup/compression support.
//! - `SurpDecoder` class: Incremental decoder with owned-value output.
//! - Custom exception hierarchy: SurpError, SurpEncodeError, SurpDecodeError

use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};

use surp_core::limits::Limits;
use surp_core::rfc001;
use surp_core::wire::CompressionType;
use surp_core::{Decoder as CoreDecoder, Encoder as CoreEncoder, Value};

use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Exception Hierarchy
// ---------------------------------------------------------------------------

create_exception!(
    _surp_native,
    SurpError,
    PyException,
    "Base exception for all Surp errors."
);
create_exception!(
    _surp_native,
    SurpEncodeError,
    SurpError,
    "Error during encoding."
);
create_exception!(
    _surp_native,
    SurpDecodeError,
    SurpError,
    "Error during decoding."
);
create_exception!(
    _surp_native,
    SurpChecksumError,
    SurpDecodeError,
    "Checksum verification failed."
);
create_exception!(
    _surp_native,
    SurpTypeError,
    SurpEncodeError,
    "Type cannot be serialized."
);
create_exception!(
    _surp_native,
    SurpRfcError,
    SurpError,
    "Error in RFC-001 CTN/CBF/CQL processing."
);

/// Map a surp_core error to our custom exception hierarchy.
fn map_decode_error(e: surp_core::SurpError) -> PyErr {
    let msg = e.to_string();
    if msg.contains("Checksum mismatch") {
        SurpChecksumError::new_err(msg)
    } else {
        SurpDecodeError::new_err(msg)
    }
}

fn map_encode_error(e: surp_core::SurpError) -> PyErr {
    SurpEncodeError::new_err(e.to_string())
}

fn map_rfc_error(e: surp_core::SurpError) -> PyErr {
    SurpRfcError::new_err(e.to_string())
}

// ---------------------------------------------------------------------------
// Python ↔ Value conversion helpers
// ---------------------------------------------------------------------------

/// Convert a Python object to a Surp `Value`.
/// If sort_keys is true, object keys will be sorted alphabetically.
fn py_to_value(obj: &Bound<'_, PyAny>, sort_keys: bool) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    // Bool must be checked before int (Python bool is a subclass of int).
    if obj.is_instance_of::<PyBool>() {
        let b: bool = obj.extract()?;
        return Ok(Value::Bool(b));
    }
    if obj.is_instance_of::<PyInt>() {
        let n: i64 = obj.extract()?;
        return if n >= 0 {
            Ok(Value::UInt(n as u64))
        } else {
            Ok(Value::Int(n))
        };
    }
    if obj.is_instance_of::<PyFloat>() {
        let f: f64 = obj.extract()?;
        return Ok(Value::Float(f));
    }
    if obj.is_instance_of::<PyString>() {
        let s: String = obj.extract()?;
        return Ok(Value::Str(s));
    }
    if obj.is_instance_of::<PyBytes>() {
        let b: Vec<u8> = obj.extract()?;
        return Ok(Value::Bytes(b));
    }
    if obj.is_instance_of::<PyDict>() {
        let d = obj.cast_exact::<PyDict>()?;
        let mut entries = Vec::with_capacity(d.len());
        for (k, v) in d.iter() {
            let key: String = k
                .extract()
                .map_err(|_| SurpTypeError::new_err("dict keys must be strings"))?;
            let val = py_to_value(&v, sort_keys)?;
            entries.push((key, val));
        }
        if sort_keys {
            entries.sort_by(|a, b| a.0.cmp(&b.0));
        }
        return Ok(Value::Object(entries));
    }
    if obj.is_instance_of::<PyList>() {
        let l = obj.cast_exact::<PyList>()?;
        let mut items = Vec::with_capacity(l.len());
        for item in l.iter() {
            items.push(py_to_value(&item, sort_keys)?);
        }
        return Ok(Value::Array(items));
    }
    if obj.is_instance_of::<PyTuple>() {
        let t = obj.cast_exact::<PyTuple>()?;
        let mut items = Vec::with_capacity(t.len());
        for item in t.iter() {
            items.push(py_to_value(&item, sort_keys)?);
        }
        return Ok(Value::Array(items));
    }
    Err(SurpTypeError::new_err(format!(
        "cannot convert {} to Surp value",
        obj.get_type().name()?
    )))
}

/// Convert a Surp `Value` to a Python object.
fn value_to_py<'py>(py: Python<'py>, value: &Value) -> PyResult<Bound<'py, PyAny>> {
    match value {
        Value::Null => Ok(py.None().into_bound(py)),
        Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any()),
        Value::UInt(n) => Ok(n.into_pyobject(py)?.into_any()),
        Value::Int(n) => Ok(n.into_pyobject(py)?.into_any()),
        Value::Float(f) => Ok(f.into_pyobject(py)?.into_any()),
        Value::Str(s) => Ok(s.into_pyobject(py)?.into_any()),
        Value::Bytes(b) => Ok(PyBytes::new(py, b).into_any()),
        Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.into_any())
        }
        Value::Object(entries) => {
            let dict = PyDict::new(py);
            for (key, val) in entries {
                dict.set_item(key, value_to_py(py, val)?)?;
            }
            Ok(dict.into_any())
        }
    }
}

/// Parse compression string to CompressionType.
fn parse_compression(comp: Option<&str>) -> PyResult<CompressionType> {
    match comp {
        None | Some("none") => Ok(CompressionType::None),
        Some("lz4") => Ok(CompressionType::Lz4),
        Some("zstd") => Ok(CompressionType::Zstd),
        Some("snappy") => Ok(CompressionType::Snappy),
        Some(other) => Err(PyValueError::new_err(format!(
            "unknown compression: {other} (expected none, lz4, zstd, snappy)"
        ))),
    }
}

// ---------------------------------------------------------------------------
// RFC-001 conversion helpers
// ---------------------------------------------------------------------------

fn rfc_scalar_to_py<'py>(py: Python<'py>, scalar: &rfc001::Scalar) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("kind", "scalar")?;
    match scalar {
        rfc001::Scalar::Null => {
            dict.set_item("type", "null")?;
            dict.set_item("value", py.None())?;
        }
        rfc001::Scalar::Unit => {
            dict.set_item("type", "unit")?;
            dict.set_item("value", py.None())?;
        }
        rfc001::Scalar::Bool(value) => {
            dict.set_item("type", "bool")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::I64(value) => {
            dict.set_item("type", "i64")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::U64(value) => {
            dict.set_item("type", "u64")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::Vi64(value) => {
            dict.set_item("type", "vi64")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::Vu64(value) => {
            dict.set_item("type", "vu64")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::F32(value) => {
            dict.set_item("type", "f32")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::F64(value) => {
            dict.set_item("type", "f64")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::Str(value) => {
            dict.set_item("type", "str")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::Bytes(value) => {
            dict.set_item("type", "bytes")?;
            dict.set_item("value", PyBytes::new(py, value))?;
        }
        rfc001::Scalar::Sym(value) => {
            dict.set_item("type", "sym")?;
            dict.set_item("value", value)?;
        }
        rfc001::Scalar::Tagged { tag, value } => {
            dict.set_item("type", "tagged")?;
            dict.set_item("tag", tag)?;
            dict.set_item("value", value)?;
        }
    }
    Ok(dict)
}

fn rfc_annotation_to_py<'py>(
    py: Python<'py>,
    annotation: &rfc001::Annotation,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("name", &annotation.name)?;
    match &annotation.value {
        Some(value) => dict.set_item("value", rfc_scalar_to_py(py, value)?)?,
        None => dict.set_item("value", py.None())?,
    }
    Ok(dict)
}

fn rfc_value_to_py<'py>(py: Python<'py>, value: &rfc001::Value) -> PyResult<Bound<'py, PyAny>> {
    match value {
        rfc001::Value::Scalar(scalar) => Ok(rfc_scalar_to_py(py, scalar)?.into_any()),
        rfc001::Value::Product(product) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", "product")?;
            dict.set_item("type_name", &product.type_name)?;
            let fields = PyList::empty(py);
            for field in &product.fields {
                let item = PyDict::new(py);
                item.set_item("name", &field.name)?;
                item.set_item("value", rfc_value_to_py(py, &field.value)?)?;
                fields.append(item)?;
            }
            dict.set_item("fields", fields)?;
            Ok(dict.into_any())
        }
        rfc001::Value::Sum(sum) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", "sum")?;
            dict.set_item("type_name", &sum.type_name)?;
            dict.set_item("variant", &sum.variant)?;
            let payload = PyDict::new(py);
            match &sum.payload {
                rfc001::SumPayload::Unit => {
                    payload.set_item("kind", "unit")?;
                }
                rfc001::SumPayload::Tuple(items) => {
                    payload.set_item("kind", "tuple")?;
                    let py_items = PyList::empty(py);
                    for item in items {
                        py_items.append(rfc_value_to_py(py, item)?)?;
                    }
                    payload.set_item("items", py_items)?;
                }
                rfc001::SumPayload::Struct(fields) => {
                    payload.set_item("kind", "struct")?;
                    let py_fields = PyList::empty(py);
                    for field in fields {
                        let py_field = PyDict::new(py);
                        py_field.set_item("name", &field.name)?;
                        py_field.set_item("value", rfc_value_to_py(py, &field.value)?)?;
                        py_fields.append(py_field)?;
                    }
                    payload.set_item("fields", py_fields)?;
                }
            }
            dict.set_item("payload", payload)?;
            Ok(dict.into_any())
        }
        rfc001::Value::Sequence(sequence) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", "sequence")?;
            dict.set_item("elem_type", &sequence.elem_type)?;
            let items = PyList::empty(py);
            for item in &sequence.items {
                items.append(rfc_value_to_py(py, item)?)?;
            }
            dict.set_item("items", items)?;
            Ok(dict.into_any())
        }
        rfc001::Value::Association(pairs) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", "association")?;
            let py_pairs = PyList::empty(py);
            for (key, value) in pairs {
                let pair = PyList::empty(py);
                pair.append(rfc_value_to_py(py, key)?)?;
                pair.append(rfc_value_to_py(py, value)?)?;
                py_pairs.append(pair)?;
            }
            dict.set_item("pairs", py_pairs)?;
            Ok(dict.into_any())
        }
        rfc001::Value::Reference(reference) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", "reference")?;
            match reference {
                rfc001::Reference::Binding(name) => {
                    dict.set_item("reference_kind", "binding")?;
                    dict.set_item("name", name)?;
                }
                rfc001::Reference::ById(inner) => {
                    dict.set_item("reference_kind", "by_id")?;
                    dict.set_item("value", rfc_value_to_py(py, inner)?)?;
                }
            }
            Ok(dict.into_any())
        }
        rfc001::Value::Tensor(tensor) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", "tensor")?;
            dict.set_item("element_type", &tensor.element_type)?;
            let shape = PyList::empty(py);
            for dim in &tensor.shape {
                match dim {
                    Some(value) => shape.append(value)?,
                    None => shape.append(py.None())?,
                }
            }
            dict.set_item("shape", shape)?;

            let annotations = PyList::empty(py);
            for annotation in &tensor.annotations {
                annotations.append(rfc_annotation_to_py(py, annotation)?)?;
            }
            dict.set_item("annotations", annotations)?;

            let data = PyDict::new(py);
            match &tensor.data {
                rfc001::TensorData::DenseF64(values) => {
                    data.set_item("kind", "dense_f64")?;
                    data.set_item("values", values)?;
                }
                rfc001::TensorData::DenseI64(values) => {
                    data.set_item("kind", "dense_i64")?;
                    data.set_item("values", values)?;
                }
                rfc001::TensorData::DenseU64(values) => {
                    data.set_item("kind", "dense_u64")?;
                    data.set_item("values", values)?;
                }
                rfc001::TensorData::BinaryBlob(bytes) => {
                    data.set_item("kind", "binary_blob")?;
                    data.set_item("bytes", PyBytes::new(py, bytes))?;
                }
            }
            dict.set_item("data", data)?;
            Ok(dict.into_any())
        }
        rfc001::Value::Stream(stream) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", "stream")?;
            dict.set_item("item_type", &stream.item_type)?;
            let annotations = PyList::empty(py);
            for annotation in &stream.annotations {
                annotations.append(rfc_annotation_to_py(py, annotation)?)?;
            }
            dict.set_item("annotations", annotations)?;
            Ok(dict.into_any())
        }
        rfc001::Value::Opaque(opaque) => {
            let dict = PyDict::new(py);
            dict.set_item("kind", "opaque")?;
            dict.set_item("type_tag", &opaque.type_tag)?;
            dict.set_item("bytes", PyBytes::new(py, &opaque.bytes))?;
            Ok(dict.into_any())
        }
    }
}

fn rfc_document_to_py<'py>(
    py: Python<'py>,
    document: &rfc001::Document,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);

    let annotations = PyList::empty(py);
    for annotation in &document.annotations {
        annotations.append(rfc_annotation_to_py(py, annotation)?)?;
    }
    dict.set_item("annotations", annotations)?;

    dict.set_item("uses", &document.uses)?;

    let bindings = PyList::empty(py);
    for binding in &document.bindings {
        let item = PyDict::new(py);
        item.set_item("name", &binding.name)?;
        item.set_item("value", rfc_value_to_py(py, &binding.value)?)?;
        bindings.append(item)?;
    }
    dict.set_item("bindings", bindings)?;

    match &document.root {
        Some(root) => dict.set_item("root", rfc_value_to_py(py, root)?)?,
        None => dict.set_item("root", py.None())?,
    }

    Ok(dict)
}

fn rfc_header_to_py<'py>(
    py: Python<'py>,
    header: &rfc001::CbfHeader,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("magic", "SURP")?;
    dict.set_item("cbf_version", header.cbf_version)?;
    dict.set_item("ctn_version", header.ctn_version)?;
    dict.set_item("flags", header.flags)?;
    dict.set_item("alignment", header.alignment)?;
    dict.set_item(
        "schema_hash_prefix",
        PyBytes::new(py, &header.schema_hash_prefix),
    )?;
    dict.set_item("root_offset", header.root_offset)?;
    dict.set_item("symtab_offset", header.symtab_offset)?;
    dict.set_item("index_offset", header.index_offset)?;
    dict.set_item("self_describing", header.self_describing())?;
    dict.set_item("has_symtab", header.has_symtab())?;
    dict.set_item("has_index", header.has_index())?;
    Ok(dict)
}

// ---------------------------------------------------------------------------
// JSON-like API: dumps / loads / dump / load
// ---------------------------------------------------------------------------

/// Serialize a Python object to Surp binary format.
///
/// This is the JSON-like API with options. Similar to ``json.dumps()``.
///
/// Args:
///     obj: The Python object to encode (dict, list, str, int, float, bytes, bool, None).
///     compression: Compression algorithm: ``None``, ``"lz4"``, ``"zstd"``, or ``"snappy"``.
///     dedup: Enable string deduplication (default: ``False``).
///     sort_keys: Sort object keys alphabetically for canonical output (default: ``False``).
///
/// Returns:
///     bytes: The encoded Surp binary data.
///
/// Raises:
///     SurpEncodeError: If encoding fails.
///     SurpTypeError: If an unsupported type is encountered.
///
/// Example::
///
///     >>> import surp
///     >>> data = surp.dumps({"hello": "world"}, compression="lz4", dedup=True)
#[pyfunction]
#[pyo3(signature = (obj, *, compression=None, dedup=false, sort_keys=false))]
fn dumps<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    compression: Option<&str>,
    dedup: bool,
    sort_keys: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    let value = py_to_value(obj, sort_keys)?;
    let mut encoder = CoreEncoder::new();

    if dedup {
        encoder.enable_dedup();
    }

    let comp_type = parse_compression(compression)?;
    encoder.set_compression(comp_type);

    encoder.encode_value(&value).map_err(map_encode_error)?;
    let bytes = encoder.finish().map_err(map_encode_error)?;
    Ok(PyBytes::new(py, &bytes))
}

/// Deserialize Surp binary data to a Python object.
///
/// This is the JSON-like API with options. Similar to ``json.loads()``.
///
/// Args:
///     data: The Surp binary data (bytes).
///     strict: If ``True`` (default), verify checksums and fail on errors.
///             If ``False``, attempt best-effort decoding.
///     max_depth: Maximum nesting depth (default: 128). Set to prevent stack overflow
///                on deeply nested data.
///
/// Returns:
///     The decoded Python object.
///
/// Raises:
///     SurpDecodeError: If decoding fails.
///     SurpChecksumError: If checksum verification fails (when strict=True).
///
/// Example::
///
///     >>> import surp
///     >>> obj = surp.loads(data)
#[pyfunction]
#[pyo3(signature = (data, *, strict=true, max_depth=128))]
fn loads<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyBytes>,
    strict: bool,
    max_depth: usize,
) -> PyResult<Bound<'py, PyAny>> {
    let buf = data.as_bytes();

    let limits = if strict {
        Limits {
            max_nesting_depth: max_depth,
            ..Limits::default()
        }
    } else {
        Limits {
            max_nesting_depth: max_depth,
            ..Limits::unlimited()
        }
    };

    let mut decoder = CoreDecoder::with_limits(buf, limits);
    let values = decoder.decode_all_owned().map_err(map_decode_error)?;

    if values.len() == 1 {
        value_to_py(py, &values[0])
    } else {
        let list = PyList::empty(py);
        for v in &values {
            list.append(value_to_py(py, v)?)?;
        }
        Ok(list.into_any())
    }
}

/// Serialize a Python object and write to a file-like object.
///
/// Similar to ``json.dump()``.
///
/// Args:
///     obj: The Python object to encode.
///     fp: A file-like object with a ``write()`` method (must accept bytes).
///     compression: Compression algorithm: ``None``, ``"lz4"``, ``"zstd"``, or ``"snappy"``.
///     dedup: Enable string deduplication (default: ``False``).
///     sort_keys: Sort object keys alphabetically (default: ``False``).
///
/// Raises:
///     SurpEncodeError: If encoding fails.
///     TypeError: If fp doesn't have a write method.
///
/// Example::
///
///     >>> with open("data.surp", "wb") as f:
///     ...     surp.dump({"hello": "world"}, f)
#[pyfunction]
#[pyo3(signature = (obj, fp, *, compression=None, dedup=false, sort_keys=false))]
fn dump(
    obj: &Bound<'_, PyAny>,
    fp: &Bound<'_, PyAny>,
    compression: Option<&str>,
    dedup: bool,
    sort_keys: bool,
) -> PyResult<()> {
    let value = py_to_value(obj, sort_keys)?;
    let mut encoder = CoreEncoder::new();

    if dedup {
        encoder.enable_dedup();
    }

    let comp_type = parse_compression(compression)?;
    encoder.set_compression(comp_type);

    encoder.encode_value(&value).map_err(map_encode_error)?;
    let bytes = encoder.finish().map_err(map_encode_error)?;

    // Call fp.write(bytes)
    let py = fp.py();
    let py_bytes = PyBytes::new(py, &bytes);
    fp.call_method1("write", (py_bytes,))?;

    Ok(())
}

/// Read and deserialize Surp binary data from a file-like object.
///
/// Similar to ``json.load()``.
///
/// Args:
///     fp: A file-like object with a ``read()`` method.
///     strict: If ``True`` (default), verify checksums.
///     max_depth: Maximum nesting depth (default: 128).
///
/// Returns:
///     The decoded Python object.
///
/// Raises:
///     SurpDecodeError: If decoding fails.
///
/// Example::
///
///     >>> with open("data.surp", "rb") as f:
///     ...     obj = surp.load(f)
#[pyfunction]
#[pyo3(signature = (fp, *, strict=true, max_depth=128))]
fn load<'py>(
    py: Python<'py>,
    fp: &Bound<'py, PyAny>,
    strict: bool,
    max_depth: usize,
) -> PyResult<Bound<'py, PyAny>> {
    // Call fp.read() to get all data
    let data_obj = fp.call_method0("read")?;
    let data: &[u8] = data_obj.extract()?;

    let limits = if strict {
        Limits {
            max_nesting_depth: max_depth,
            ..Limits::default()
        }
    } else {
        Limits {
            max_nesting_depth: max_depth,
            ..Limits::unlimited()
        }
    };

    let mut decoder = CoreDecoder::with_limits(data, limits);
    let values = decoder.decode_all_owned().map_err(map_decode_error)?;

    if values.len() == 1 {
        value_to_py(py, &values[0])
    } else {
        let list = PyList::empty(py);
        for v in &values {
            list.append(value_to_py(py, v)?)?;
        }
        Ok(list.into_any())
    }
}

// ---------------------------------------------------------------------------
// Legacy API (kept for backward compatibility)
// ---------------------------------------------------------------------------

/// Encode a Python object (dict, list, str, int, float, bytes, bool, None)
/// into Surp binary format, returned as `bytes`.
///
/// This is the simple API. For more options, use ``dumps()``.
#[pyfunction]
fn encode<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyBytes>> {
    dumps(py, obj, None, false, false)
}

/// Decode Surp binary `bytes` into Python objects.
///
/// Returns a single value if the data contains exactly one top-level value,
/// or a list of values otherwise.
///
/// This is the simple API. For more options, use ``loads()``.
#[pyfunction]
fn decode<'py>(py: Python<'py>, data: &Bound<'py, PyBytes>) -> PyResult<Bound<'py, PyAny>> {
    loads(py, data, true, 128)
}

/// Encode a Python object and write the binary output to a file.
///
/// Args:
///     obj: The Python object to encode (dict, list, str, int, float, bytes, bool, None).
///     path: File path to write the encoded binary data to.
///
/// Raises:
///     SurpEncodeError: If encoding fails.
///     OSError: If writing to the file fails.
#[pyfunction]
fn encode_to_file(obj: &Bound<'_, PyAny>, path: PathBuf) -> PyResult<()> {
    let value = py_to_value(obj, false)?;
    let mut encoder = CoreEncoder::new();
    encoder.encode_value(&value).map_err(map_encode_error)?;
    let bytes = encoder.finish().map_err(map_encode_error)?;
    fs::write(path, &bytes)?;
    Ok(())
}

/// Read a Surp binary file and decode it into Python objects.
///
/// Args:
///     path: File path to read the binary data from.
///
/// Returns:
///     The decoded Python object (single value or list of values).
///
/// Raises:
///     SurpDecodeError: If decoding fails.
///     OSError: If reading the file fails.
#[pyfunction]
fn decode_from_file<'py>(py: Python<'py>, path: PathBuf) -> PyResult<Bound<'py, PyAny>> {
    let buf = fs::read(path)?;
    let mut decoder = CoreDecoder::new(&buf);
    let values = decoder.decode_all_owned().map_err(map_decode_error)?;

    if values.len() == 1 {
        value_to_py(py, &values[0])
    } else {
        let list = PyList::empty(py);
        for v in &values {
            list.append(value_to_py(py, v)?)?;
        }
        Ok(list.into_any())
    }
}

/// Parse Surp human-readable text notation into a Python object.
///
/// Args:
///     text: A string in Surp text format.
///
/// Returns:
///     The parsed Python object.
///
/// Raises:
///     ValueError: If the text cannot be parsed.
#[pyfunction]
fn parse_text<'py>(py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyAny>> {
    let value = surp_core::text::parse(text)
        .map_err(|e| PyValueError::new_err(format!("parse error: {e}")))?;
    value_to_py(py, &value)
}

/// Pretty-print a Python object in Surp human-readable text notation.
///
/// Args:
///     obj: The Python object to format.
///     indent: Number of spaces per indentation level (default: 2).
///
/// Returns:
///     A string in Surp text format.
///
/// Raises:
///     SurpTypeError: If the object cannot be converted to a Surp value.
#[pyfunction]
#[pyo3(signature = (obj, indent=2))]
fn pretty_print(obj: &Bound<'_, PyAny>, indent: usize) -> PyResult<String> {
    let value = py_to_value(obj, false)?;
    Ok(surp_core::text::pretty_print(&value, indent))
}

// ---------------------------------------------------------------------------
// RFC-001 API (CTN + CBF + CQL)
// ---------------------------------------------------------------------------

/// Parse an RFC-001 CTN document into a typed Python dictionary.
#[pyfunction]
fn rfc_parse_ctn<'py>(py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyDict>> {
    let document = rfc001::parse_document(text).map_err(map_rfc_error)?;
    rfc_document_to_py(py, &document)
}

/// Parse and format an RFC-001 CTN document.
#[pyfunction]
fn rfc_format_ctn(text: &str) -> PyResult<String> {
    let document = rfc001::parse_document(text).map_err(map_rfc_error)?;
    Ok(rfc001::format_document(&document))
}

/// Compile RFC-001 CTN text to CBF bytes.
#[pyfunction]
#[pyo3(signature = (text, *, with_symtab=true, alignment=0))]
fn rfc_compile_ctn<'py>(
    py: Python<'py>,
    text: &str,
    with_symtab: bool,
    alignment: u8,
) -> PyResult<Bound<'py, PyBytes>> {
    let document = rfc001::parse_document(text).map_err(map_rfc_error)?;
    let bytes = rfc001::encode_document(
        &document,
        rfc001::EncodeOptions {
            with_symtab,
            alignment,
        },
    )
    .map_err(map_rfc_error)?;
    Ok(PyBytes::new(py, &bytes))
}

/// Decode RFC-001 CBF bytes and return header, symbols, document, and CTN.
#[pyfunction]
fn rfc_decode_cbf<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyBytes>,
) -> PyResult<Bound<'py, PyDict>> {
    let decoded = rfc001::decode_document(data.as_bytes()).map_err(map_rfc_error)?;
    let dict = PyDict::new(py);
    dict.set_item("header", rfc_header_to_py(py, &decoded.header)?)?;
    dict.set_item("symbols", decoded.symbols)?;
    dict.set_item("document", rfc_document_to_py(py, &decoded.document)?)?;
    dict.set_item("ctn", rfc001::format_document(&decoded.document))?;
    Ok(dict)
}

/// Decode RFC-001 CBF bytes to CTN text.
#[pyfunction]
fn rfc_cbf_to_ctn(data: &Bound<'_, PyBytes>) -> PyResult<String> {
    let decoded = rfc001::decode_document(data.as_bytes()).map_err(map_rfc_error)?;
    Ok(rfc001::format_document(&decoded.document))
}

/// Execute a baseline RFC-001 CQL path query over CBF bytes.
#[pyfunction]
#[pyo3(signature = (data, query, *, as_ctn=false))]
fn rfc_query_cbf<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyBytes>,
    query: &str,
    as_ctn: bool,
) -> PyResult<Bound<'py, PyList>> {
    let decoded = rfc001::decode_document(data.as_bytes()).map_err(map_rfc_error)?;
    let root = decoded.document.effective_root().map_err(map_rfc_error)?;
    let results = rfc001::query(&root, query).map_err(map_rfc_error)?;
    rfc_results_to_py(py, &results, as_ctn)
}

/// Execute a baseline RFC-001 CQL path query over CTN text.
#[pyfunction]
#[pyo3(signature = (text, query, *, as_ctn=false))]
fn rfc_query_ctn<'py>(
    py: Python<'py>,
    text: &str,
    query: &str,
    as_ctn: bool,
) -> PyResult<Bound<'py, PyList>> {
    let document = rfc001::parse_document(text).map_err(map_rfc_error)?;
    let bytes = rfc001::encode_document(&document, rfc001::EncodeOptions::default())
        .map_err(map_rfc_error)?;
    let decoded = rfc001::decode_document(&bytes).map_err(map_rfc_error)?;
    let root = decoded.document.effective_root().map_err(map_rfc_error)?;
    let results = rfc001::query(&root, query).map_err(map_rfc_error)?;
    rfc_results_to_py(py, &results, as_ctn)
}

fn rfc_results_to_py<'py>(
    py: Python<'py>,
    results: &[rfc001::Value],
    as_ctn: bool,
) -> PyResult<Bound<'py, PyList>> {
    let list = PyList::empty(py);
    for value in results {
        if as_ctn {
            list.append(rfc001::format_value(value))?;
        } else {
            list.append(rfc_value_to_py(py, value)?)?;
        }
    }
    Ok(list)
}

// ---------------------------------------------------------------------------
// Encoder class
// ---------------------------------------------------------------------------

/// Incremental Surp encoder.
///
/// Example::
///
///     enc = Encoder()
///     enc.enable_dedup()
///     enc.set_compression("lz4")
///     enc.encode({"key": "value"})
///     data = enc.finish()
#[pyclass]
struct Encoder {
    inner: Option<CoreEncoder>,
    sort_keys: bool,
}

#[pymethods]
impl Encoder {
    #[new]
    #[pyo3(signature = (*, sort_keys=false))]
    fn new(sort_keys: bool) -> Self {
        Self {
            inner: Some(CoreEncoder::new()),
            sort_keys,
        }
    }

    /// Enable string deduplication for subsequent blocks.
    fn enable_dedup(&mut self) -> PyResult<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| SurpEncodeError::new_err("encoder already finished"))?
            .enable_dedup();
        Ok(())
    }

    /// Set compression type: "none", "lz4", "zstd", or "snappy".
    fn set_compression(&mut self, comp: &str) -> PyResult<()> {
        let ct = parse_compression(Some(comp))?;
        self.inner
            .as_mut()
            .ok_or_else(|| SurpEncodeError::new_err("encoder already finished"))?
            .set_compression(ct);
        Ok(())
    }

    /// Encode a Python value into the current block.
    fn encode(&mut self, obj: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = py_to_value(obj, self.sort_keys)?;
        self.inner
            .as_mut()
            .ok_or_else(|| SurpEncodeError::new_err("encoder already finished"))?
            .encode_value(&value)
            .map_err(map_encode_error)?;
        Ok(())
    }

    /// Flush and finalize the encoder, returning the Surp binary output as `bytes`.
    /// The encoder cannot be used after this call.
    fn finish<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let enc = self
            .inner
            .take()
            .ok_or_else(|| SurpEncodeError::new_err("encoder already finished"))?;
        let bytes = enc.finish().map_err(map_encode_error)?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Flush and finalize the encoder, writing the output directly to a file.
    /// The encoder cannot be used after this call.
    ///
    /// Args:
    ///     path: File path to write the encoded binary data to.
    ///
    /// Raises:
    ///     SurpEncodeError: If encoding fails or the encoder was already finished.
    ///     OSError: If writing to the file fails.
    fn finish_to_file(&mut self, path: PathBuf) -> PyResult<()> {
        let enc = self
            .inner
            .take()
            .ok_or_else(|| SurpEncodeError::new_err("encoder already finished"))?;
        let bytes = enc.finish().map_err(map_encode_error)?;
        fs::write(path, &bytes)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Decoder class
// ---------------------------------------------------------------------------

/// Incremental Surp decoder.
///
/// Example::
///
///     dec = SurpDecoder(data)
///     values = dec.decode_all()
#[pyclass]
struct SurpDecoder {
    /// We store the data so the decoder can borrow from it.
    data: Vec<u8>,
    /// Whether decode_all has been called.
    consumed: bool,
    /// Custom limits.
    max_depth: usize,
}

#[pymethods]
impl SurpDecoder {
    #[new]
    #[pyo3(signature = (data, *, max_depth=128))]
    fn new(data: &Bound<'_, PyBytes>, max_depth: usize) -> Self {
        Self {
            data: data.as_bytes().to_vec(),
            consumed: false,
            max_depth,
        }
    }

    /// Decode all values from the binary data.
    fn decode_all<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        if self.consumed {
            return Err(SurpDecodeError::new_err("decoder already consumed"));
        }
        self.consumed = true;

        let limits = Limits {
            max_nesting_depth: self.max_depth,
            ..Limits::default()
        };

        let mut decoder = CoreDecoder::with_limits(&self.data, limits);
        let values = decoder.decode_all_owned().map_err(map_decode_error)?;

        let list = PyList::empty(py);
        for v in &values {
            list.append(value_to_py(py, v)?)?;
        }
        Ok(list)
    }
}

// ---------------------------------------------------------------------------
// Module definition
// ---------------------------------------------------------------------------

/// surp native extension (Rust-backed via PyO3).
///
/// Provides high-performance encode/decode for the Surp binary format.
///
/// JSON-like API::
///
///     import surp
///
///     # Encode with options
///     data = surp.dumps(obj, compression="lz4", dedup=True, sort_keys=True)
///
///     # Decode with options
///     obj = surp.loads(data, strict=True, max_depth=64)
///
///     # File I/O
///     with open("data.surp", "wb") as f:
///         surp.dump(obj, f)
///
///     with open("data.surp", "rb") as f:
///         obj = surp.load(f)
#[pymodule]
fn _surp_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", "1.0.0")?;

    // Exception hierarchy
    m.add("SurpError", m.py().get_type::<SurpError>())?;
    m.add("SurpEncodeError", m.py().get_type::<SurpEncodeError>())?;
    m.add("SurpDecodeError", m.py().get_type::<SurpDecodeError>())?;
    m.add("SurpChecksumError", m.py().get_type::<SurpChecksumError>())?;
    m.add("SurpTypeError", m.py().get_type::<SurpTypeError>())?;
    m.add("SurpRfcError", m.py().get_type::<SurpRfcError>())?;

    // JSON-like API
    m.add_function(wrap_pyfunction!(dumps, m)?)?;
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    m.add_function(wrap_pyfunction!(dump, m)?)?;
    m.add_function(wrap_pyfunction!(load, m)?)?;

    // Legacy API (backward compatible)
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(encode_to_file, m)?)?;
    m.add_function(wrap_pyfunction!(decode_from_file, m)?)?;

    // Text format
    m.add_function(wrap_pyfunction!(parse_text, m)?)?;
    m.add_function(wrap_pyfunction!(pretty_print, m)?)?;

    // RFC-001 CTN / CBF / CQL
    m.add("RFC001_CBF_MAGIC", PyBytes::new(m.py(), &rfc001::CBF_MAGIC))?;
    m.add("RFC001_CBF_HEADER_SIZE", rfc001::CBF_HEADER_SIZE)?;
    m.add_function(wrap_pyfunction!(rfc_parse_ctn, m)?)?;
    m.add_function(wrap_pyfunction!(rfc_format_ctn, m)?)?;
    m.add_function(wrap_pyfunction!(rfc_compile_ctn, m)?)?;
    m.add_function(wrap_pyfunction!(rfc_decode_cbf, m)?)?;
    m.add_function(wrap_pyfunction!(rfc_cbf_to_ctn, m)?)?;
    m.add_function(wrap_pyfunction!(rfc_query_cbf, m)?)?;
    m.add_function(wrap_pyfunction!(rfc_query_ctn, m)?)?;

    // Classes
    m.add_class::<Encoder>()?;
    m.add_class::<SurpDecoder>()?;

    Ok(())
}
