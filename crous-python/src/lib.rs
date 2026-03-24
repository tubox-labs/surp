//! # crous-python
//!
//! PyO3 native extension that wraps `crous-core` encoder/decoder for Python.
//!
//! Provides:
//! - `encode(obj)` / `decode(data)` - Simple encode/decode functions
//! - `dumps(obj, ...)` / `loads(data, ...)` - JSON-like API with options
//! - `dump(obj, fp, ...)` / `load(fp, ...)` - File-based JSON-like API
//! - `Encoder` class: Incremental encoder with dedup/compression support.
//! - `CrousDecoder` class: Incremental decoder with owned-value output.
//! - Custom exception hierarchy: CrousError, CrousEncodeError, CrousDecodeError

use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString};

use crous_core::limits::Limits;
use crous_core::wire::CompressionType;
use crous_core::{Decoder as CoreDecoder, Encoder as CoreEncoder, Value};

use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Exception Hierarchy
// ---------------------------------------------------------------------------

create_exception!(_crous_native, CrousError, PyException, "Base exception for all Crous errors.");
create_exception!(_crous_native, CrousEncodeError, CrousError, "Error during encoding.");
create_exception!(_crous_native, CrousDecodeError, CrousError, "Error during decoding.");
create_exception!(_crous_native, CrousChecksumError, CrousDecodeError, "Checksum verification failed.");
create_exception!(_crous_native, CrousTypeError, CrousEncodeError, "Type cannot be serialized.");

/// Map a crous_core error to our custom exception hierarchy.
fn map_decode_error(e: crous_core::CrousError) -> PyErr {
    let msg = e.to_string();
    if msg.contains("Checksum mismatch") {
        CrousChecksumError::new_err(msg)
    } else {
        CrousDecodeError::new_err(msg)
    }
}

fn map_encode_error(e: crous_core::CrousError) -> PyErr {
    CrousEncodeError::new_err(e.to_string())
}

// ---------------------------------------------------------------------------
// Python ↔ Value conversion helpers
// ---------------------------------------------------------------------------

/// Convert a Python object to a Crous `Value`.
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
                .map_err(|_| CrousTypeError::new_err("dict keys must be strings"))?;
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
    Err(CrousTypeError::new_err(format!(
        "cannot convert {} to Crous value",
        obj.get_type().name()?
    )))
}

/// Convert a Crous `Value` to a Python object.
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
// JSON-like API: dumps / loads / dump / load
// ---------------------------------------------------------------------------

/// Serialize a Python object to Crous binary format.
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
///     bytes: The encoded Crous binary data.
///
/// Raises:
///     CrousEncodeError: If encoding fails.
///     CrousTypeError: If an unsupported type is encountered.
///
/// Example::
///
///     >>> import crous
///     >>> data = crous.dumps({"hello": "world"}, compression="lz4", dedup=True)
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

/// Deserialize Crous binary data to a Python object.
///
/// This is the JSON-like API with options. Similar to ``json.loads()``.
///
/// Args:
///     data: The Crous binary data (bytes).
///     strict: If ``True`` (default), verify checksums and fail on errors.
///             If ``False``, attempt best-effort decoding.
///     max_depth: Maximum nesting depth (default: 128). Set to prevent stack overflow
///                on deeply nested data.
///
/// Returns:
///     The decoded Python object.
///
/// Raises:
///     CrousDecodeError: If decoding fails.
///     CrousChecksumError: If checksum verification fails (when strict=True).
///
/// Example::
///
///     >>> import crous
///     >>> obj = crous.loads(data)
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
///     CrousEncodeError: If encoding fails.
///     TypeError: If fp doesn't have a write method.
///
/// Example::
///
///     >>> with open("data.crous", "wb") as f:
///     ...     crous.dump({"hello": "world"}, f)
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

/// Read and deserialize Crous binary data from a file-like object.
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
///     CrousDecodeError: If decoding fails.
///
/// Example::
///
///     >>> with open("data.crous", "rb") as f:
///     ...     obj = crous.load(f)
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
/// into Crous binary format, returned as `bytes`.
///
/// This is the simple API. For more options, use ``dumps()``.
#[pyfunction]
fn encode<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyBytes>> {
    dumps(py, obj, None, false, false)
}

/// Decode Crous binary `bytes` into Python objects.
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
///     CrousEncodeError: If encoding fails.
///     OSError: If writing to the file fails.
#[pyfunction]
fn encode_to_file(obj: &Bound<'_, PyAny>, path: &str) -> PyResult<()> {
    let value = py_to_value(obj, false)?;
    let mut encoder = CoreEncoder::new();
    encoder.encode_value(&value).map_err(map_encode_error)?;
    let bytes = encoder.finish().map_err(map_encode_error)?;
    fs::write(PathBuf::from(path), &bytes)?;
    Ok(())
}

/// Read a Crous binary file and decode it into Python objects.
///
/// Args:
///     path: File path to read the binary data from.
///
/// Returns:
///     The decoded Python object (single value or list of values).
///
/// Raises:
///     CrousDecodeError: If decoding fails.
///     OSError: If reading the file fails.
#[pyfunction]
fn decode_from_file<'py>(py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyAny>> {
    let buf = fs::read(PathBuf::from(path))?;
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

/// Parse Crous human-readable text notation into a Python object.
///
/// Args:
///     text: A string in Crous text format.
///
/// Returns:
///     The parsed Python object.
///
/// Raises:
///     ValueError: If the text cannot be parsed.
#[pyfunction]
fn parse_text<'py>(py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyAny>> {
    let value = crous_core::text::parse(text)
        .map_err(|e| PyValueError::new_err(format!("parse error: {e}")))?;
    value_to_py(py, &value)
}

/// Pretty-print a Python object in Crous human-readable text notation.
///
/// Args:
///     obj: The Python object to format.
///     indent: Number of spaces per indentation level (default: 2).
///
/// Returns:
///     A string in Crous text format.
///
/// Raises:
///     CrousTypeError: If the object cannot be converted to a Crous value.
#[pyfunction]
#[pyo3(signature = (obj, indent=2))]
fn pretty_print(obj: &Bound<'_, PyAny>, indent: usize) -> PyResult<String> {
    let value = py_to_value(obj, false)?;
    Ok(crous_core::text::pretty_print(&value, indent))
}

// ---------------------------------------------------------------------------
// Encoder class
// ---------------------------------------------------------------------------

/// Incremental Crous encoder.
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
            .ok_or_else(|| CrousEncodeError::new_err("encoder already finished"))?
            .enable_dedup();
        Ok(())
    }

    /// Set compression type: "none", "lz4", "zstd", or "snappy".
    fn set_compression(&mut self, comp: &str) -> PyResult<()> {
        let ct = parse_compression(Some(comp))?;
        self.inner
            .as_mut()
            .ok_or_else(|| CrousEncodeError::new_err("encoder already finished"))?
            .set_compression(ct);
        Ok(())
    }

    /// Encode a Python value into the current block.
    fn encode(&mut self, obj: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = py_to_value(obj, self.sort_keys)?;
        self.inner
            .as_mut()
            .ok_or_else(|| CrousEncodeError::new_err("encoder already finished"))?
            .encode_value(&value)
            .map_err(map_encode_error)?;
        Ok(())
    }

    /// Flush and finalize the encoder, returning the Crous binary output as `bytes`.
    /// The encoder cannot be used after this call.
    fn finish<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let enc = self
            .inner
            .take()
            .ok_or_else(|| CrousEncodeError::new_err("encoder already finished"))?;
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
    ///     CrousEncodeError: If encoding fails or the encoder was already finished.
    ///     OSError: If writing to the file fails.
    fn finish_to_file(&mut self, path: &str) -> PyResult<()> {
        let enc = self
            .inner
            .take()
            .ok_or_else(|| CrousEncodeError::new_err("encoder already finished"))?;
        let bytes = enc.finish().map_err(map_encode_error)?;
        fs::write(PathBuf::from(path), &bytes)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Decoder class
// ---------------------------------------------------------------------------

/// Incremental Crous decoder.
///
/// Example::
///
///     dec = CrousDecoder(data)
///     values = dec.decode_all()
#[pyclass]
struct CrousDecoder {
    /// We store the data so the decoder can borrow from it.
    data: Vec<u8>,
    /// Whether decode_all has been called.
    consumed: bool,
    /// Custom limits.
    max_depth: usize,
}

#[pymethods]
impl CrousDecoder {
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
            return Err(CrousDecodeError::new_err("decoder already consumed"));
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

/// crous native extension (Rust-backed via PyO3).
///
/// Provides high-performance encode/decode for the Crous binary format.
///
/// JSON-like API::
///
///     import crous
///
///     # Encode with options
///     data = crous.dumps(obj, compression="lz4", dedup=True, sort_keys=True)
///
///     # Decode with options
///     obj = crous.loads(data, strict=True, max_depth=64)
///
///     # File I/O
///     with open("data.crous", "wb") as f:
///         crous.dump(obj, f)
///
///     with open("data.crous", "rb") as f:
///         obj = crous.load(f)
#[pymodule]
fn _crous_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", "1.2.0")?;

    // Exception hierarchy
    m.add("CrousError", m.py().get_type::<CrousError>())?;
    m.add("CrousEncodeError", m.py().get_type::<CrousEncodeError>())?;
    m.add("CrousDecodeError", m.py().get_type::<CrousDecodeError>())?;
    m.add("CrousChecksumError", m.py().get_type::<CrousChecksumError>())?;
    m.add("CrousTypeError", m.py().get_type::<CrousTypeError>())?;

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

    // Classes
    m.add_class::<Encoder>()?;
    m.add_class::<CrousDecoder>()?;

    Ok(())
}
