//! RFC-001 CTN parser and formatter.
//!
//! The parser targets the RFC-001 indentation-oriented syntax and supports
//! a substantial executable subset of the draft:
//! - document annotations (`@...`)
//! - `let` bindings
//! - struct/product blocks
//! - enum/sum variants
//! - sequences, maps, references
//! - typed scalar literals and tagged literals (e.g. `ts"..."`)
//! - tensor and stream headers

use crate::error::{Result, SurpError};
use base64::Engine;

use super::ast::{
    Annotation, Binding, Document, Field, Opaque, Product, Reference, Scalar, Sequence, Stream,
    Sum, SumPayload, Tensor, TensorData, Value,
};

/// Parse a complete CTN document.
pub fn parse_document(input: &str) -> Result<Document> {
    let lines = preprocess_lines(input)?;
    let parser = Parser { lines };
    parser.parse_document()
}

/// Parse a single value expression.
pub fn parse_value(input: &str) -> Result<Value> {
    let doc = parse_document(input)?;
    doc.effective_root()
}

/// Render a full CTN document.
pub fn format_document(doc: &Document) -> String {
    let mut out = String::new();
    for ann in &doc.annotations {
        out.push_str(&format_annotation(ann));
        out.push('\n');
    }
    for use_stmt in &doc.uses {
        out.push_str("use ");
        out.push_str(use_stmt);
        out.push('\n');
    }
    for binding in &doc.bindings {
        if let Some(inline) = to_inline(&binding.value) {
            out.push_str("let ");
            out.push_str(&binding.name);
            out.push_str(" = ");
            out.push_str(&inline);
            out.push('\n');
        } else {
            out.push_str("let ");
            out.push_str(&binding.name);
            out.push_str(" = ");
            let rendered = format_value(&binding.value);
            let mut rendered_lines = rendered.lines();
            if let Some(first) = rendered_lines.next() {
                out.push_str(first);
                out.push('\n');
            }
            for line in rendered_lines {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    if let Some(root) = &doc.root {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format_value(root));
    } else if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Render a single CTN value.
pub fn format_value(value: &Value) -> String {
    render_value(value, 0)
}

#[derive(Debug, Clone)]
struct Line {
    number: usize,
    indent: usize,
    text: String,
}

struct Parser {
    lines: Vec<Line>,
}

impl Parser {
    fn parse_document(&self) -> Result<Document> {
        let mut idx = 0usize;
        let mut doc = Document::default();

        while idx < self.lines.len() {
            let line = &self.lines[idx];
            let text = line.text.trim();

            if text.starts_with('@') {
                doc.annotations.push(parse_annotation_line(text)?);
                idx += 1;
                continue;
            }

            if let Some(rest) = text.strip_prefix("use ") {
                doc.uses.push(rest.trim().to_string());
                idx += 1;
                continue;
            }

            if let Some(rest) = text.strip_prefix("let ") {
                let (name, rhs) = split_once_required(rest, "=", line.number)?;
                let name = name.trim();
                if name.is_empty() || !is_identifier(name) {
                    return Err(parse_err(
                        line.number,
                        format!("invalid binding name '{name}'"),
                    ));
                }
                let (value, next) = self.parse_rhs_or_nested(rhs.trim(), idx, line.indent)?;
                doc.bindings.push(Binding {
                    name: name.to_string(),
                    value,
                });
                idx = next;
                continue;
            }

            let (value, next) = self.parse_value_at(idx)?;
            doc.root = Some(value);
            idx = next;
        }

        Ok(doc)
    }

    fn parse_value_at(&self, idx: usize) -> Result<(Value, usize)> {
        let line = self
            .lines
            .get(idx)
            .ok_or_else(|| parse_err(0, "internal parser index out of bounds".into()))?;
        self.parse_value_from_header(line.text.trim(), idx, line.indent)
    }

    fn parse_rhs_or_nested(
        &self,
        rhs: &str,
        line_idx: usize,
        line_indent: usize,
    ) -> Result<(Value, usize)> {
        if rhs.is_empty() {
            let Some(next) = self.lines.get(line_idx + 1) else {
                return Err(parse_err(
                    self.lines[line_idx].number,
                    "expected value after '='".into(),
                ));
            };
            if next.indent <= line_indent {
                return Err(parse_err(
                    self.lines[line_idx].number,
                    "expected indented value after '='".into(),
                ));
            }
            return self.parse_value_at(line_idx + 1);
        }

        if self.next_line_is_child(line_idx, line_indent) && header_supports_block(rhs) {
            return self.parse_value_from_header(rhs, line_idx, line_indent);
        }

        Ok((parse_inline_expr(rhs)?, line_idx + 1))
    }

    fn parse_value_from_header(
        &self,
        header: &str,
        line_idx: usize,
        line_indent: usize,
    ) -> Result<(Value, usize)> {
        let text = header.trim();

        if text.starts_with("map<")
            && !text.contains('[')
            && self.next_line_is_child(line_idx, line_indent)
        {
            return self.parse_map_block(text, line_idx, line_indent);
        }

        if text.starts_with("seq<") && self.next_line_is_child(line_idx, line_indent) {
            return self.parse_sequence_block(text, line_idx, line_indent);
        }

        if (text.starts_with("vec<") || text.starts_with("tensor<") || text.starts_with("mat<"))
            && self.next_line_is_child(line_idx, line_indent)
        {
            return self.parse_tensor_block(text, line_idx, line_indent);
        }

        if text.starts_with("stream<") {
            return self.parse_stream_block(text, line_idx, line_indent);
        }

        if text.contains("::")
            && self.next_line_is_child(line_idx, line_indent)
            && !text.contains('(')
        {
            return self.parse_sum_struct_block(text, line_idx, line_indent);
        }

        if is_type_header(text) && self.has_field_children(line_idx, line_indent) {
            return self.parse_struct_block(text, line_idx, line_indent);
        }

        let value = parse_inline_expr(text)?;
        Ok((value, line_idx + 1))
    }

    fn parse_struct_block(
        &self,
        header: &str,
        line_idx: usize,
        parent_indent: usize,
    ) -> Result<(Value, usize)> {
        let type_name = if header.trim() == "struct" {
            None
        } else {
            Some(header.trim().to_string())
        };

        let mut idx = line_idx + 1;
        let child_indent = self
            .lines
            .get(idx)
            .ok_or_else(|| parse_err(self.lines[line_idx].number, "expected fields".into()))?
            .indent;

        let mut fields = Vec::new();
        while idx < self.lines.len() && self.lines[idx].indent > parent_indent {
            if self.lines[idx].indent != child_indent {
                return Err(parse_err(
                    self.lines[idx].number,
                    "inconsistent indentation in struct fields".into(),
                ));
            }
            let line = &self.lines[idx];
            let (name, rhs) = split_once_required(&line.text, "=", line.number)?;
            let field_name = name.trim();
            if field_name.is_empty() || !is_identifier(field_name) {
                return Err(parse_err(
                    line.number,
                    format!("invalid field name '{field_name}'"),
                ));
            }
            let (value, next) = self.parse_rhs_or_nested(rhs.trim(), idx, line.indent)?;
            fields.push(Field {
                name: field_name.to_string(),
                value,
            });
            idx = next;
        }

        Ok((Value::Product(Product { type_name, fields }), idx))
    }

    fn parse_sum_struct_block(
        &self,
        header: &str,
        line_idx: usize,
        parent_indent: usize,
    ) -> Result<(Value, usize)> {
        let (left, right) = split_once_required(header, "::", self.lines[line_idx].number)?;
        let type_name = {
            let t = left.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        };

        let variant = right.trim().to_string();
        if variant.is_empty() {
            return Err(parse_err(
                self.lines[line_idx].number,
                "missing enum variant name".into(),
            ));
        }

        let mut idx = line_idx + 1;
        let child_indent = self
            .lines
            .get(idx)
            .ok_or_else(|| {
                parse_err(
                    self.lines[line_idx].number,
                    "expected variant payload".into(),
                )
            })?
            .indent;

        let mut fields = Vec::new();
        while idx < self.lines.len() && self.lines[idx].indent > parent_indent {
            if self.lines[idx].indent != child_indent {
                return Err(parse_err(
                    self.lines[idx].number,
                    "inconsistent indentation in enum payload".into(),
                ));
            }
            let line = &self.lines[idx];
            let (name, rhs) = split_once_required(&line.text, "=", line.number)?;
            let field_name = name.trim();
            if field_name.is_empty() || !is_identifier(field_name) {
                return Err(parse_err(
                    line.number,
                    format!("invalid payload field '{field_name}'"),
                ));
            }
            let (value, next) = self.parse_rhs_or_nested(rhs.trim(), idx, line.indent)?;
            fields.push(Field {
                name: field_name.to_string(),
                value,
            });
            idx = next;
        }

        Ok((
            Value::Sum(Sum {
                type_name,
                variant,
                payload: SumPayload::Struct(fields),
            }),
            idx,
        ))
    }

    fn parse_sequence_block(
        &self,
        header: &str,
        line_idx: usize,
        parent_indent: usize,
    ) -> Result<(Value, usize)> {
        let elem_type = parse_generic_type_arg(header);
        let mut idx = line_idx + 1;
        let child_indent = self
            .lines
            .get(idx)
            .ok_or_else(|| {
                parse_err(
                    self.lines[line_idx].number,
                    "expected sequence elements".into(),
                )
            })?
            .indent;

        let mut items = Vec::new();
        while idx < self.lines.len() && self.lines[idx].indent > parent_indent {
            if self.lines[idx].indent != child_indent {
                return Err(parse_err(
                    self.lines[idx].number,
                    "inconsistent indentation in sequence".into(),
                ));
            }
            let (value, next) = self.parse_value_at(idx)?;
            items.push(value);
            idx = next;
        }

        Ok((Value::Sequence(Sequence { elem_type, items }), idx))
    }

    fn parse_map_block(
        &self,
        _header: &str,
        line_idx: usize,
        parent_indent: usize,
    ) -> Result<(Value, usize)> {
        let mut idx = line_idx + 1;
        let child_indent = self
            .lines
            .get(idx)
            .ok_or_else(|| parse_err(self.lines[line_idx].number, "expected map entries".into()))?
            .indent;

        let mut pairs = Vec::new();
        while idx < self.lines.len() && self.lines[idx].indent > parent_indent {
            if self.lines[idx].indent != child_indent {
                return Err(parse_err(
                    self.lines[idx].number,
                    "inconsistent indentation in map".into(),
                ));
            }

            let line = &self.lines[idx];
            let (lhs, rhs) = split_once_required(&line.text, "=>", line.number)?;
            let key = parse_inline_expr(lhs.trim())?;
            let (value, next) = self.parse_rhs_or_nested(rhs.trim(), idx, line.indent)?;
            pairs.push((key, value));
            idx = next;
        }

        Ok((Value::Association(pairs), idx))
    }

    fn parse_stream_block(
        &self,
        header: &str,
        line_idx: usize,
        parent_indent: usize,
    ) -> Result<(Value, usize)> {
        let item_type = parse_generic_type_arg(header).unwrap_or_else(|| "any".to_string());

        let mut annotations = Vec::new();
        let mut idx = line_idx + 1;
        while idx < self.lines.len() && self.lines[idx].indent > parent_indent {
            let line = &self.lines[idx];
            if !line.text.trim().starts_with('@') {
                return Err(parse_err(
                    line.number,
                    "stream body currently supports annotations only".into(),
                ));
            }
            annotations.push(parse_annotation_line(line.text.trim())?);
            idx += 1;
        }

        Ok((
            Value::Stream(Stream {
                item_type,
                annotations,
            }),
            idx,
        ))
    }

    fn parse_tensor_block(
        &self,
        header: &str,
        line_idx: usize,
        parent_indent: usize,
    ) -> Result<(Value, usize)> {
        let element_type = parse_generic_type_arg(header).unwrap_or_else(|| "f32".to_string());
        let shape = parse_shape(header)?;

        let mut idx = line_idx + 1;
        let mut annotations = Vec::new();
        let mut dense = Vec::new();
        let mut binary_blob: Option<Vec<u8>> = None;

        while idx < self.lines.len() && self.lines[idx].indent > parent_indent {
            let line = &self.lines[idx];
            let text = line.text.trim();
            if text.starts_with('@') {
                annotations.push(parse_annotation_line(text)?);
                idx += 1;
                continue;
            }

            if let Some(bytes) = parse_binary_placeholder(text) {
                binary_blob = Some(vec![0u8; bytes]);
                idx += 1;
                continue;
            }

            let value = parse_inline_expr(text)?;
            collect_numeric_scalars(&value, &mut dense)?;
            idx += 1;
        }

        let data = if let Some(blob) = binary_blob {
            TensorData::BinaryBlob(blob)
        } else {
            TensorData::DenseF64(dense)
        };

        Ok((
            Value::Tensor(Tensor {
                element_type,
                shape,
                data,
                annotations,
            }),
            idx,
        ))
    }

    fn next_line_is_child(&self, idx: usize, indent: usize) -> bool {
        self.lines
            .get(idx + 1)
            .is_some_and(|line| line.indent > indent)
    }

    fn has_field_children(&self, idx: usize, indent: usize) -> bool {
        self.lines
            .get(idx + 1)
            .is_some_and(|line| line.indent > indent && line.text.contains('='))
    }
}

fn preprocess_lines(input: &str) -> Result<Vec<Line>> {
    let mut lines = Vec::new();
    let mut in_block_comment = false;

    for (line_idx, raw_line) in input.lines().enumerate() {
        let mut line = raw_line.to_string();

        if in_block_comment {
            if let Some(end) = line.find("]]") {
                line = line[end + 2..].to_string();
                in_block_comment = false;
            } else {
                continue;
            }
        }

        loop {
            if let Some(start) = find_unquoted(&line, "--[[") {
                let prefix = line[..start].to_string();
                let rest = &line[start + 4..];
                if let Some(end) = find_unquoted(rest, "]]") {
                    let suffix = &rest[end + 2..];
                    line = format!("{prefix}{suffix}");
                    continue;
                }
                line = prefix;
                in_block_comment = true;
            }
            break;
        }

        if line.trim_start().starts_with("--!") {
            continue;
        }

        let stripped = strip_line_comment(&line);
        if stripped.trim().is_empty() {
            continue;
        }

        if stripped.starts_with('\t') {
            return Err(parse_err(
                line_idx + 1,
                "tabs are not allowed for indentation in CTN".into(),
            ));
        }

        let indent = stripped.chars().take_while(|c| *c == ' ').count();
        let text = stripped[indent..].trim_end().to_string();
        if text.is_empty() {
            continue;
        }

        lines.push(Line {
            number: line_idx + 1,
            indent,
            text,
        });
    }

    Ok(lines)
}

fn strip_line_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    let mut idx = 0usize;
    let bytes = line.as_bytes();

    while idx + 1 < bytes.len() {
        let ch = bytes[idx] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            idx += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            idx += 1;
            continue;
        }

        if bytes[idx] == b'-' && bytes[idx + 1] == b'-' {
            return &line[..idx];
        }
        idx += 1;
    }

    line
}

fn parse_inline_expr(input: &str) -> Result<Value> {
    let s = input.trim();
    if s.is_empty() {
        return Err(SurpError::InvalidData("empty inline expression".into()));
    }

    if s == "null" {
        return Ok(Value::Scalar(Scalar::Null));
    }
    if s == "unit" {
        return Ok(Value::Scalar(Scalar::Unit));
    }
    if s == "true" {
        return Ok(Value::Scalar(Scalar::Bool(true)));
    }
    if s == "false" {
        return Ok(Value::Scalar(Scalar::Bool(false)));
    }

    if let Some(name) = s.strip_prefix('&') {
        let name = name.trim();
        if name.is_empty() || !is_identifier(name) {
            return Err(SurpError::InvalidData(format!(
                "invalid reference name '{name}'"
            )));
        }
        return Ok(Value::Reference(Reference::Binding(name.to_string())));
    }

    if let Some(rest) = s.strip_prefix("ref ") {
        let value = parse_inline_expr(rest.trim())?;
        return Ok(Value::Reference(Reference::ById(Box::new(value))));
    }

    if s.starts_with("\"\"\"") {
        return Ok(Value::Scalar(Scalar::Str(parse_triple_quoted_string(s)?)));
    }

    if s.starts_with('"') {
        return Ok(Value::Scalar(Scalar::Str(parse_quoted_string(s)?)));
    }

    if let Some(b64) = s.strip_prefix("b64\"") {
        if !s.ends_with('"') {
            return Err(SurpError::InvalidData(
                "unterminated base64 bytes literal".into(),
            ));
        }
        let payload = &b64[..b64.len() - 1];
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|e| SurpError::InvalidBase64(e.to_string()))?;
        return Ok(Value::Scalar(Scalar::Bytes(bytes)));
    }

    if let Some(hex) = s.strip_prefix("b\"") {
        if !s.ends_with('"') {
            return Err(SurpError::InvalidData(
                "unterminated hex bytes literal".into(),
            ));
        }
        let payload = &hex[..hex.len() - 1];
        return Ok(Value::Scalar(Scalar::Bytes(parse_hex_compact(payload)?)));
    }

    if s.starts_with('<') && s.ends_with('>') {
        let payload = &s[1..s.len() - 1];
        return Ok(Value::Scalar(Scalar::Bytes(parse_hex_spaced(payload)?)));
    }

    if let Some((tag, value)) = parse_prefixed_quoted(s)? {
        return Ok(Value::Scalar(Scalar::Tagged { tag, value }));
    }

    if let Some(sym) = s.strip_prefix('\'') {
        if sym.is_empty() || !is_identifier(sym) {
            return Err(SurpError::InvalidData(format!("invalid symbol '{sym}'")));
        }
        return Ok(Value::Scalar(Scalar::Sym(sym.to_string())));
    }

    if s.starts_with("map<") && s.contains('[') && s.ends_with(']') {
        return parse_inline_map(s);
    }

    if s.starts_with('[') && s.ends_with(']') {
        return parse_inline_sequence(s);
    }

    if s.starts_with('(') && s.ends_with(')') {
        let inside = &s[1..s.len() - 1];
        if inside.trim().is_empty() {
            return Ok(Value::Sequence(Sequence {
                elem_type: None,
                items: Vec::new(),
            }));
        }
        let mut items = Vec::new();
        for token in split_top_level(inside, ',') {
            items.push(parse_inline_expr(token.trim())?);
        }
        return Ok(Value::Sequence(Sequence {
            elem_type: None,
            items,
        }));
    }

    if s.contains("::") {
        return parse_inline_sum(s);
    }

    if let Some(num) = parse_numeric_literal(s)? {
        return Ok(Value::Scalar(num));
    }

    if is_identifier_path(s) {
        return Ok(Value::Scalar(Scalar::Sym(s.to_string())));
    }

    Err(SurpError::InvalidData(format!(
        "unable to parse inline expression '{s}'"
    )))
}

fn parse_inline_sum(s: &str) -> Result<Value> {
    let (left, right) = split_once_required(s, "::", 0)?;
    let type_name = {
        let t = left.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    };

    let rhs = right.trim();
    if rhs.is_empty() {
        return Err(SurpError::InvalidData("missing enum variant".into()));
    }

    if let Some(open) = rhs.find('(') {
        if !rhs.ends_with(')') {
            return Err(SurpError::InvalidData("unterminated enum payload".into()));
        }
        let variant = rhs[..open].trim().to_string();
        let payload_src = &rhs[open + 1..rhs.len() - 1];
        let parts = split_top_level(payload_src, ',');

        if parts.iter().any(|p| p.contains(':')) {
            let mut fields = Vec::new();
            for part in parts {
                let (name, value) = split_once_required(part, ":", 0)?;
                fields.push(Field {
                    name: name.trim().to_string(),
                    value: parse_inline_expr(value.trim())?,
                });
            }
            return Ok(Value::Sum(Sum {
                type_name,
                variant,
                payload: SumPayload::Struct(fields),
            }));
        }

        let mut tuple_items = Vec::new();
        for part in parts {
            if part.trim().is_empty() {
                continue;
            }
            tuple_items.push(parse_inline_expr(part.trim())?);
        }
        return Ok(Value::Sum(Sum {
            type_name,
            variant,
            payload: SumPayload::Tuple(tuple_items),
        }));
    }

    Ok(Value::Sum(Sum {
        type_name,
        variant: rhs.to_string(),
        payload: SumPayload::Unit,
    }))
}

fn parse_inline_sequence(s: &str) -> Result<Value> {
    let inside = &s[1..s.len() - 1];
    let tokens = split_top_level(inside, ',');
    let mut items = Vec::new();
    for token in tokens {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        items.push(parse_inline_expr(t)?);
    }

    Ok(Value::Sequence(Sequence {
        elem_type: None,
        items,
    }))
}

fn parse_inline_map(s: &str) -> Result<Value> {
    let open = s
        .find('[')
        .ok_or_else(|| SurpError::InvalidData("invalid inline map".into()))?;
    let inside = &s[open + 1..s.len() - 1];
    let tokens = split_top_level(inside, ',');

    let mut pairs = Vec::new();
    for token in tokens {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let (lhs, rhs) = split_once_required(token, "=>", 0)?;
        pairs.push((
            parse_inline_expr(lhs.trim())?,
            parse_inline_expr(rhs.trim())?,
        ));
    }

    Ok(Value::Association(pairs))
}

fn parse_numeric_literal(s: &str) -> Result<Option<Scalar>> {
    let suffixes = [
        "dec128", "dec64", "dec32", "bf16", "f128", "f64", "f32", "f16", "vi64", "vi32", "vu64",
        "vu32", "i128", "i64", "i32", "i16", "i8", "u128", "u64", "u32", "u16", "u8",
    ];

    let mut suffix = "";
    let mut body = s;
    for candidate in suffixes {
        if let Some(prefix) = s.strip_suffix(candidate) {
            suffix = candidate;
            body = prefix;
            break;
        }
    }

    let body = body.trim();
    if body.is_empty() {
        return Ok(None);
    }

    if !looks_like_numeric(body) {
        return Ok(None);
    }

    let lowered = body.to_ascii_lowercase();
    if lowered == "nan" {
        return Ok(Some(match suffix {
            "f32" => Scalar::F32(f32::NAN),
            _ => Scalar::F64(f64::NAN),
        }));
    }

    if body == "Inf" || body == "+Inf" || lowered == "inf" {
        return Ok(Some(match suffix {
            "f32" => Scalar::F32(f32::INFINITY),
            _ => Scalar::F64(f64::INFINITY),
        }));
    }

    if body == "-Inf" || lowered == "-inf" {
        return Ok(Some(match suffix {
            "f32" => Scalar::F32(f32::NEG_INFINITY),
            _ => Scalar::F64(f64::NEG_INFINITY),
        }));
    }

    if suffix.starts_with("dec") {
        return Ok(Some(Scalar::Tagged {
            tag: suffix.to_string(),
            value: body.to_string(),
        }));
    }

    if body.contains('.') || body.contains('e') || body.contains('E') {
        let parsed = parse_float_text(body)?;
        return Ok(Some(match suffix {
            "f32" | "f16" | "bf16" => Scalar::F32(parsed as f32),
            "f64" | "f128" | "" => Scalar::F64(parsed),
            _ => Scalar::Tagged {
                tag: suffix.to_string(),
                value: body.to_string(),
            },
        }));
    }

    if matches!(suffix, "f32" | "f16" | "bf16" | "f64" | "f128") {
        let parsed = parse_float_text(body)?;
        return Ok(Some(match suffix {
            "f32" | "f16" | "bf16" => Scalar::F32(parsed as f32),
            "f64" | "f128" => Scalar::F64(parsed),
            _ => unreachable!("float suffix match is exhaustive"),
        }));
    }

    let value_i128 = parse_integer_text(body)?;

    let scalar = match suffix {
        "u8" | "u16" | "u32" | "u64" | "u128" | "vu32" | "vu64" => {
            if value_i128 < 0 {
                return Err(SurpError::InvalidData(format!(
                    "unsigned literal cannot be negative: {s}"
                )));
            }
            let value = u64::try_from(value_i128).map_err(|_| {
                SurpError::InvalidData(format!("unsigned literal out of range: {s}"))
            })?;
            if suffix.starts_with("vu") {
                Scalar::Vu64(value)
            } else {
                Scalar::U64(value)
            }
        }
        "i8" | "i16" | "i32" | "i64" | "i128" => {
            let value = i64::try_from(value_i128)
                .map_err(|_| SurpError::InvalidData(format!("signed literal out of range: {s}")))?;
            Scalar::I64(value)
        }
        "vi32" | "vi64" | "" => {
            let value = i64::try_from(value_i128)
                .map_err(|_| SurpError::InvalidData(format!("signed literal out of range: {s}")))?;
            Scalar::Vi64(value)
        }
        other => Scalar::Tagged {
            tag: other.to_string(),
            value: body.to_string(),
        },
    };

    Ok(Some(scalar))
}

fn parse_float_text(text: &str) -> Result<f64> {
    let cleaned = text.replace('_', "");
    cleaned
        .parse::<f64>()
        .map_err(|_| SurpError::InvalidData(format!("invalid float literal '{text}'")))
}

fn looks_like_numeric(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    if matches!(text, "NaN" | "Inf" | "+Inf" | "-Inf") {
        return true;
    }

    if text.starts_with("0x")
        || text.starts_with("-0x")
        || text.starts_with("+0x")
        || text.starts_with("0b")
        || text.starts_with("-0b")
        || text.starts_with("+0b")
        || text.starts_with("0o")
        || text.starts_with("-0o")
        || text.starts_with("+0o")
    {
        return true;
    }

    let first = text.as_bytes()[0] as char;
    if !(first.is_ascii_digit() || first == '+' || first == '-' || first == '.') {
        return false;
    }

    text.chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E' | '_'))
}

fn parse_integer_text(text: &str) -> Result<i128> {
    let mut s = text.trim();
    let sign = if let Some(rest) = s.strip_prefix('-') {
        s = rest;
        -1i128
    } else if let Some(rest) = s.strip_prefix('+') {
        s = rest;
        1i128
    } else {
        1i128
    };

    let (radix, digits) = if let Some(rest) = s.strip_prefix("0x") {
        (16u32, rest)
    } else if let Some(rest) = s.strip_prefix("0b") {
        (2u32, rest)
    } else if let Some(rest) = s.strip_prefix("0o") {
        (8u32, rest)
    } else {
        (10u32, s)
    };

    let digits = digits.replace('_', "");
    if digits.is_empty() {
        return Err(SurpError::InvalidData(format!(
            "invalid integer literal '{text}'"
        )));
    }

    let magnitude = i128::from_str_radix(&digits, radix)
        .map_err(|_| SurpError::InvalidData(format!("invalid integer literal '{text}'")))?;

    Ok(sign * magnitude)
}

fn parse_prefixed_quoted(s: &str) -> Result<Option<(String, String)>> {
    let Some(first_quote) = s.find('"') else {
        return Ok(None);
    };
    if !s.ends_with('"') || first_quote == 0 {
        return Ok(None);
    }

    let tag = s[..first_quote].trim();
    if !is_identifier(tag) {
        return Ok(None);
    }

    let value = parse_quoted_string(&s[first_quote..])?;
    Ok(Some((tag.to_string(), value)))
}

fn parse_quoted_string(s: &str) -> Result<String> {
    if !s.starts_with('"') || !s.ends_with('"') || s.len() < 2 {
        return Err(SurpError::InvalidData(
            "invalid quoted string literal".into(),
        ));
    }

    let mut out = String::new();
    let mut chars = s[1..s.len() - 1].chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        let Some(esc) = chars.next() else {
            return Err(SurpError::InvalidData(
                "unterminated escape sequence".into(),
            ));
        };

        match esc {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'b' => out.push('\u{0008}'),
            'f' => out.push('\u{000C}'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            'u' => {
                if chars.next() != Some('{') {
                    return Err(SurpError::InvalidData(
                        "expected '{' in unicode escape".into(),
                    ));
                }
                let mut hex = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    hex.push(c);
                }
                let code = u32::from_str_radix(&hex, 16)
                    .map_err(|_| SurpError::InvalidData("invalid unicode escape".into()))?;
                let ch = char::from_u32(code)
                    .ok_or_else(|| SurpError::InvalidData("invalid unicode scalar value".into()))?;
                out.push(ch);
            }
            other => {
                return Err(SurpError::InvalidData(format!(
                    "unsupported escape sequence \\{other}"
                )));
            }
        }
    }

    Ok(out)
}

fn parse_triple_quoted_string(s: &str) -> Result<String> {
    if !s.starts_with("\"\"\"") || !s.ends_with("\"\"\"") || s.len() < 6 {
        return Err(SurpError::InvalidData(
            "invalid triple-quoted string literal".into(),
        ));
    }
    let inner = &s[3..s.len() - 3];
    Ok(inner.to_string())
}

fn parse_hex_compact(hex: &str) -> Result<Vec<u8>> {
    let payload = hex.replace('_', "");
    if payload.len() % 2 != 0 {
        return Err(SurpError::InvalidData(
            "hex bytes literal must have even length".into(),
        ));
    }
    let mut out = Vec::with_capacity(payload.len() / 2);
    let bytes = payload.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let pair = std::str::from_utf8(&bytes[idx..idx + 2])
            .map_err(|_| SurpError::InvalidData("invalid hex bytes literal".into()))?;
        let byte = u8::from_str_radix(pair, 16)
            .map_err(|_| SurpError::InvalidData("invalid hex bytes literal".into()))?;
        out.push(byte);
        idx += 2;
    }
    Ok(out)
}

fn parse_hex_spaced(hex: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for token in hex.split_whitespace() {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if token.len() != 2 {
            return Err(SurpError::InvalidData(
                "space-separated hex bytes must be two hex digits each".into(),
            ));
        }
        let byte = u8::from_str_radix(token, 16)
            .map_err(|_| SurpError::InvalidData("invalid hex byte in literal".into()))?;
        out.push(byte);
    }
    Ok(out)
}

fn parse_annotation_line(text: &str) -> Result<Annotation> {
    let body = text.trim().trim_start_matches('@').trim();
    if body.is_empty() {
        return Err(SurpError::InvalidData("annotation name missing".into()));
    }

    let mut iter = body.splitn(2, char::is_whitespace);
    let name = iter.next().unwrap_or_default();
    if !is_identifier(name) {
        return Err(SurpError::InvalidData(format!(
            "invalid annotation name '{name}'"
        )));
    }

    let value = iter
        .next()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| {
            parse_inline_expr(v).and_then(|val| match val {
                Value::Scalar(s) => Ok(s),
                _ => Err(SurpError::InvalidData(
                    "annotation values must be scalar".into(),
                )),
            })
        })
        .transpose()?;

    Ok(Annotation {
        name: name.to_string(),
        value,
    })
}

fn parse_shape(header: &str) -> Result<Vec<Option<u64>>> {
    let Some(open) = header.find('[') else {
        return Ok(Vec::new());
    };
    let Some(close) = header.rfind(']') else {
        return Err(SurpError::InvalidData("unterminated tensor shape".into()));
    };
    if close <= open {
        return Ok(Vec::new());
    }
    let inside = &header[open + 1..close];
    if inside.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for token in split_top_level(inside, ',') {
        let token = token.trim();
        if token == "_" {
            out.push(None);
        } else {
            let scalar = parse_numeric_literal(token)?
                .ok_or_else(|| SurpError::InvalidData("invalid tensor dimension".into()))?;
            let Some(raw) = scalar.as_i64() else {
                return Err(SurpError::InvalidData(
                    "tensor dimensions must be integers".into(),
                ));
            };
            if raw < 0 {
                return Err(SurpError::InvalidData(
                    "tensor dimensions cannot be negative".into(),
                ));
            }
            out.push(Some(raw as u64));
        }
    }
    Ok(out)
}

fn parse_binary_placeholder(text: &str) -> Option<usize> {
    let body = text.trim();
    let body = body.strip_prefix("<binary:")?;
    let body = body.strip_suffix('>')?;
    let body = body.trim();
    let body = body.strip_suffix("bytes")?.trim();
    body.parse::<usize>().ok()
}

fn collect_numeric_scalars(value: &Value, out: &mut Vec<f64>) -> Result<()> {
    match value {
        Value::Scalar(s) => {
            let Some(v) = s.as_f64() else {
                return Err(SurpError::InvalidData(
                    "tensor literal contains non-numeric scalar".into(),
                ));
            };
            out.push(v);
            Ok(())
        }
        Value::Sequence(seq) => {
            for item in &seq.items {
                collect_numeric_scalars(item, out)?;
            }
            Ok(())
        }
        _ => Err(SurpError::InvalidData(
            "tensor literal must contain only numeric sequences".into(),
        )),
    }
}

fn parse_generic_type_arg(header: &str) -> Option<String> {
    let open = header.find('<')?;
    let close = header.rfind('>')?;
    if close <= open {
        return None;
    }
    Some(header[open + 1..close].trim().to_string())
}

fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_angle = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '<' => depth_angle += 1,
            '>' => depth_angle = depth_angle.saturating_sub(1),
            _ => {}
        }

        if ch == delimiter && depth_paren == 0 && depth_bracket == 0 && depth_angle == 0 {
            parts.push(input[start..idx].trim());
            start = idx + ch.len_utf8();
        }
    }

    if start <= input.len() {
        parts.push(input[start..].trim());
    }

    parts
}

fn split_once_required(input: &str, sep: impl AsRef<str>, line: usize) -> Result<(&str, &str)> {
    let sep = sep.as_ref();
    let Some((lhs, rhs)) = input.split_once(sep) else {
        return Err(parse_err(
            line,
            format!("expected separator '{sep}' in '{input}'"),
        ));
    };
    Ok((lhs, rhs))
}

fn is_type_header(s: &str) -> bool {
    s == "struct" || is_identifier_path(s)
}

fn header_supports_block(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("map<")
        || s.starts_with("seq<")
        || s.starts_with("vec<")
        || s.starts_with("tensor<")
        || s.starts_with("mat<")
        || s.starts_with("stream<")
        || s.contains("::")
        || is_type_header(s)
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_identifier_path(s: &str) -> bool {
    s.split("::").all(is_identifier)
}

fn find_unquoted(haystack: &str, needle: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    let bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut idx = 0usize;

    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            idx += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            idx += 1;
            continue;
        }

        if idx + needle_bytes.len() <= bytes.len()
            && &bytes[idx..idx + needle_bytes.len()] == needle_bytes
        {
            return Some(idx);
        }

        idx += 1;
    }

    None
}

fn parse_err(line: usize, message: String) -> SurpError {
    SurpError::ParseError {
        line: if line == 0 { 1 } else { line },
        col: 1,
        message,
    }
}

fn format_annotation(ann: &Annotation) -> String {
    if let Some(value) = &ann.value {
        format!("@{} {}", ann.name, format_scalar(value))
    } else {
        format!("@{}", ann.name)
    }
}

fn render_value(value: &Value, indent: usize) -> String {
    if let Some(inline) = to_inline(value) {
        return inline;
    }

    let child_pad = " ".repeat(indent + 2);

    match value {
        Value::Product(product) => {
            let mut out = String::new();
            out.push_str(product.type_name.as_deref().unwrap_or("struct"));
            for field in &product.fields {
                if let Some(inline) = to_inline(&field.value) {
                    out.push('\n');
                    out.push_str(&child_pad);
                    out.push_str(&field.name);
                    out.push_str(" = ");
                    out.push_str(&inline);
                } else {
                    out.push('\n');
                    out.push_str(&child_pad);
                    out.push_str(&field.name);
                    out.push_str(" = ");
                    let nested = render_value(&field.value, indent + 2);
                    let mut lines = nested.lines();
                    if let Some(first) = lines.next() {
                        out.push_str(first);
                    }
                    for line in lines {
                        out.push('\n');
                        out.push_str(line);
                    }
                }
            }
            out
        }
        Value::Sequence(seq) => {
            let mut out = String::new();
            let type_name = seq.elem_type.as_deref().unwrap_or("any");
            out.push_str("seq<");
            out.push_str(type_name);
            out.push('>');
            for item in &seq.items {
                out.push('\n');
                out.push_str(&child_pad);
                let nested = render_value(item, indent + 2);
                let mut lines = nested.lines();
                if let Some(first) = lines.next() {
                    out.push_str(first);
                }
                for line in lines {
                    out.push('\n');
                    out.push_str(line);
                }
            }
            out
        }
        Value::Association(map) => {
            let mut out = String::new();
            out.push_str("map<any, any>");
            if map.is_empty() {
                out.push_str(" []");
                return out;
            }
            for (key, value) in map {
                out.push('\n');
                out.push_str(&child_pad);
                out.push_str(&render_value(key, indent + 2));
                out.push_str(" => ");
                if let Some(inline) = to_inline(value) {
                    out.push_str(&inline);
                } else {
                    let nested = render_value(value, indent + 2);
                    let mut lines = nested.lines();
                    if let Some(first) = lines.next() {
                        out.push_str(first);
                    }
                    for line in lines {
                        out.push('\n');
                        out.push_str(line);
                    }
                }
            }
            out
        }
        Value::Sum(sum) => {
            let mut out = String::new();
            if let Some(t) = &sum.type_name {
                out.push_str(t);
                out.push_str(" :: ");
            }
            out.push_str(&sum.variant);
            match &sum.payload {
                SumPayload::Unit => {}
                SumPayload::Tuple(items) => {
                    out.push('(');
                    for (idx, item) in items.iter().enumerate() {
                        if idx > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&render_value(item, indent));
                    }
                    out.push(')');
                }
                SumPayload::Struct(fields) => {
                    for field in fields {
                        out.push('\n');
                        out.push_str(&child_pad);
                        out.push_str(&field.name);
                        out.push_str(" = ");
                        out.push_str(&render_value(&field.value, indent + 2));
                    }
                }
            }
            out
        }
        Value::Tensor(tensor) => {
            let mut out = String::new();
            out.push_str("tensor<");
            out.push_str(&tensor.element_type);
            out.push('>');
            if !tensor.shape.is_empty() {
                out.push('[');
                for (idx, dim) in tensor.shape.iter().enumerate() {
                    if idx > 0 {
                        out.push_str(", ");
                    }
                    match dim {
                        Some(v) => out.push_str(&v.to_string()),
                        None => out.push('_'),
                    }
                }
                out.push(']');
            }
            for ann in &tensor.annotations {
                out.push('\n');
                out.push_str(&child_pad);
                out.push_str(&format_annotation(ann));
            }
            match &tensor.data {
                TensorData::DenseF64(values) => {
                    out.push('\n');
                    out.push_str(&child_pad);
                    out.push('[');
                    for (idx, val) in values.iter().enumerate() {
                        if idx > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&format!("{val}f64"));
                    }
                    out.push(']');
                }
                TensorData::DenseI64(values) => {
                    out.push('\n');
                    out.push_str(&child_pad);
                    out.push('[');
                    for (idx, val) in values.iter().enumerate() {
                        if idx > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&format!("{val}i64"));
                    }
                    out.push(']');
                }
                TensorData::DenseU64(values) => {
                    out.push('\n');
                    out.push_str(&child_pad);
                    out.push('[');
                    for (idx, val) in values.iter().enumerate() {
                        if idx > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&format!("{val}u64"));
                    }
                    out.push(']');
                }
                TensorData::BinaryBlob(bytes) => {
                    out.push('\n');
                    out.push_str(&child_pad);
                    out.push_str(&format!("<binary: {} bytes>", bytes.len()));
                }
            }
            out
        }
        Value::Stream(stream) => {
            let mut out = String::new();
            out.push_str("stream<");
            out.push_str(&stream.item_type);
            out.push('>');
            for ann in &stream.annotations {
                out.push('\n');
                out.push_str(&child_pad);
                out.push_str(&format_annotation(ann));
            }
            out
        }
        Value::Reference(reference) => match reference {
            Reference::Binding(name) => format!("&{name}"),
            Reference::ById(value) => format!("ref {}", render_value(value, indent)),
        },
        Value::Opaque(Opaque { type_tag, bytes }) => {
            format!("opaque<{type_tag}> <binary: {} bytes>", bytes.len())
        }
        Value::Scalar(s) => format_scalar(s),
    }
}

fn to_inline(value: &Value) -> Option<String> {
    match value {
        Value::Scalar(s) => Some(format_scalar(s)),
        Value::Reference(Reference::Binding(name)) => Some(format!("&{name}")),
        Value::Reference(Reference::ById(value)) => Some(format!("ref {}", render_value(value, 0))),
        Value::Sum(sum) if matches!(sum.payload, SumPayload::Unit) => {
            if let Some(t) = &sum.type_name {
                Some(format!("{} :: {}", t, sum.variant))
            } else {
                Some(sum.variant.clone())
            }
        }
        Value::Sequence(seq)
            if seq.items.len() <= 8
                && seq
                    .items
                    .iter()
                    .all(|item| matches!(item, Value::Scalar(_) | Value::Reference(_))) =>
        {
            let mut out = String::new();
            out.push('[');
            for (idx, item) in seq.items.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                out.push_str(&render_value(item, 0));
            }
            out.push(']');
            Some(out)
        }
        _ => None,
    }
}

fn format_scalar(s: &Scalar) -> String {
    match s {
        Scalar::Null => "null".into(),
        Scalar::Unit => "unit".into(),
        Scalar::Bool(v) => {
            if *v {
                "true".into()
            } else {
                "false".into()
            }
        }
        Scalar::I64(v) => format!("{v}i64"),
        Scalar::U64(v) => format!("{v}u64"),
        Scalar::Vi64(v) => v.to_string(),
        Scalar::Vu64(v) => v.to_string(),
        Scalar::F32(v) => format!("{v}f32"),
        Scalar::F64(v) => format!("{v}f64"),
        Scalar::Str(v) => format!("\"{}\"", escape_string(v)),
        Scalar::Bytes(v) => format!(
            "b64\"{}\"",
            base64::engine::general_purpose::STANDARD.encode(v)
        ),
        Scalar::Sym(v) => format!("'{v}"),
        Scalar::Tagged { tag, value } => {
            format!("{tag}\"{}\"", escape_string(value))
        }
    }
}

fn escape_string(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_struct_with_nested_values() {
        let input = r#"
User
  id = uid"550e8400-e29b-41d4-a716-446655440000"
  age = 30u8
  active = true
  tags = ['admin, 'staff]
"#;

        let value = parse_value(input).unwrap();
        let Value::Product(product) = value else {
            panic!("expected struct product");
        };
        assert_eq!(product.type_name.as_deref(), Some("User"));
        assert_eq!(product.fields.len(), 4);
    }

    #[test]
    fn parses_document_with_annotations_and_binding() {
        let input = r#"
@surp v1
@encoding cbf

let alice = User
  id = uid"550e8400-e29b-41d4-a716-446655440000"
  name = "Alice"

&alice
"#;

        let doc = parse_document(input).unwrap();
        assert_eq!(doc.annotations.len(), 2);
        assert_eq!(doc.bindings.len(), 1);
        assert!(matches!(doc.root, Some(Value::Reference(_))));
    }

    #[test]
    fn parses_inline_map_and_sequence() {
        let value =
            parse_value("map<str, bool> [\"enabled\" => true, \"debug\" => false]").unwrap();
        let Value::Association(pairs) = value else {
            panic!("expected map association");
        };
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn empty_map_format_is_parseable_and_idempotent() {
        let value = parse_value("map<str, str> []").unwrap();
        let text = format_value(&value);
        assert_eq!(text, "map<any, any> []");
        let reparsed = parse_value(&text).unwrap();
        assert_eq!(value, reparsed);
        assert_eq!(format_value(&reparsed), text);
    }

    #[test]
    fn format_roundtrip_preserves_shape() {
        let input = r#"
Order
  id = uid"a1b2c3d4-e5f6-7890-abcd-ef1234567890"
  total = 99.99f64
  items = ["a", "b"]
"#;
        let value = parse_value(input).unwrap();
        let text = format_value(&value);
        let reparsed = parse_value(&text).unwrap();
        assert_eq!(value, reparsed);
    }

    #[test]
    fn parses_tensor_block() {
        let input = r#"
tensor<f32>[2, 2]
  [1.0f32, 2.0f32]
  [3.0f32, 4.0f32]
"#;
        let value = parse_value(input).unwrap();
        let Value::Tensor(tensor) = value else {
            panic!("expected tensor");
        };
        assert_eq!(tensor.shape.len(), 2);
        match tensor.data {
            TensorData::DenseF64(values) => assert_eq!(values.len(), 4),
            _ => panic!("expected dense f64 tensor data"),
        }
    }
}
