//! RFC-001 CQL baseline path engine.
//!
//! This is a foundational executable subset of CQL focused on structural
//! traversal over in-memory RFC-001 values.

use crate::error::{Result, SurpError};

use super::ast::{Reference, SumPayload, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Field(String),
    MapKey(String),
    Flatten,
    Index(isize),
}

/// Execute a baseline CQL path query.
///
/// Supported syntax examples:
/// - `.user.email`
/// - `.orders[]`
/// - `.orders[0]`
/// - `.settings['theme]`
/// - `.metadata["created_at"]`
pub fn query(root: &Value, expr: &str) -> Result<Vec<Value>> {
    let steps = parse_steps(expr)?;

    let mut nodes = vec![root.clone()];
    for step in steps {
        let mut next = Vec::new();
        for node in &nodes {
            apply_step(node, &step, &mut next)?;
        }
        nodes = next;
    }

    Ok(nodes)
}

/// Execute a query and return the first value, if any.
pub fn query_one(root: &Value, expr: &str) -> Result<Option<Value>> {
    Ok(query(root, expr)?.into_iter().next())
}

fn parse_steps(expr: &str) -> Result<Vec<Step>> {
    let mut s = expr.trim();
    if s.is_empty() {
        return Err(SurpError::InvalidData("empty CQL expression".into()));
    }
    if !s.starts_with('.') {
        return Err(SurpError::InvalidData(
            "CQL expression must start with '.'".into(),
        ));
    }

    let mut steps = Vec::new();
    while !s.is_empty() {
        if let Some(rest) = s.strip_prefix('.') {
            s = rest;
            let mut end = 0usize;
            for (idx, ch) in s.char_indices() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    end = idx + ch.len_utf8();
                } else {
                    break;
                }
            }
            if end == 0 {
                continue;
            }
            steps.push(Step::Field(s[..end].to_string()));
            s = &s[end..];
            continue;
        }

        if s.starts_with('[') {
            let end = find_matching_bracket(s)?;
            let inner = s[1..end].trim();
            if inner.is_empty() {
                steps.push(Step::Flatten);
            } else if let Ok(index) = inner.parse::<isize>() {
                steps.push(Step::Index(index));
            } else if inner.starts_with('"') && inner.ends_with('"') {
                let key = inner[1..inner.len() - 1].to_string();
                steps.push(Step::MapKey(key));
            } else if let Some(sym) = inner.strip_prefix('\'') {
                steps.push(Step::MapKey(sym.to_string()));
            } else {
                return Err(SurpError::InvalidData(format!(
                    "unsupported CQL bracket selector [{inner}]"
                )));
            }
            s = &s[end + 1..];
            continue;
        }

        return Err(SurpError::InvalidData(format!(
            "unexpected CQL token near '{s}'"
        )));
    }

    Ok(steps)
}

fn find_matching_bracket(s: &str) -> Result<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in s.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                if depth == 0 {
                    return Err(SurpError::InvalidData("unbalanced bracket in CQL".into()));
                }
                depth -= 1;
                if depth == 0 {
                    return Ok(idx);
                }
            }
            _ => {}
        }
    }

    Err(SurpError::InvalidData(
        "unterminated bracket in CQL expression".into(),
    ))
}

fn apply_step(node: &Value, step: &Step, out: &mut Vec<Value>) -> Result<()> {
    match step {
        Step::Field(name) => match node {
            Value::Product(product) => {
                if let Some(value) = product.field(name) {
                    out.push(value.clone());
                }
            }
            Value::Association(map) => {
                for (key, value) in map {
                    if map_key_eq(key, name) {
                        out.push(value.clone());
                    }
                }
            }
            Value::Sum(sum) => {
                if let SumPayload::Struct(fields) = &sum.payload {
                    if let Some(field) = fields.iter().find(|f| f.name == *name) {
                        out.push(field.value.clone());
                    }
                }
            }
            Value::Reference(Reference::ById(inner)) => apply_step(inner, step, out)?,
            _ => {}
        },
        Step::MapKey(name) => match node {
            Value::Association(map) => {
                for (key, value) in map {
                    if map_key_eq(key, name) {
                        out.push(value.clone());
                    }
                }
            }
            Value::Product(product) => {
                if let Some(value) = product.field(name) {
                    out.push(value.clone());
                }
            }
            Value::Reference(Reference::ById(inner)) => apply_step(inner, step, out)?,
            _ => {}
        },
        Step::Flatten => match node {
            Value::Sequence(seq) => {
                for item in &seq.items {
                    out.push(item.clone());
                }
            }
            Value::Association(map) => {
                for (_key, value) in map {
                    out.push(value.clone());
                }
            }
            Value::Reference(Reference::ById(inner)) => apply_step(inner, step, out)?,
            _ => {}
        },
        Step::Index(index) => match node {
            Value::Sequence(seq) => {
                if seq.items.is_empty() {
                    return Ok(());
                }
                let len = seq.items.len() as isize;
                let idx = if *index < 0 { len + *index } else { *index };
                if idx >= 0 {
                    let idx =
                        usize::try_from(idx).map_err(|_| SurpError::LengthOverflow(idx as u64))?;
                    if idx < seq.items.len() {
                        out.push(seq.items[idx].clone());
                    }
                }
            }
            Value::Reference(Reference::ById(inner)) => apply_step(inner, step, out)?,
            _ => {}
        },
    }

    Ok(())
}

fn map_key_eq(key: &Value, expected: &str) -> bool {
    match key {
        Value::Scalar(super::ast::Scalar::Str(s)) => s == expected,
        Value::Scalar(super::ast::Scalar::Sym(s)) => s == expected,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rfc001::ast::{Field, Product, Scalar, Sequence, Value};

    fn sample_root() -> Value {
        Value::Product(Product {
            type_name: Some("Root".into()),
            fields: vec![
                Field {
                    name: "user".into(),
                    value: Value::Product(Product {
                        type_name: Some("User".into()),
                        fields: vec![
                            Field {
                                name: "email".into(),
                                value: Value::Scalar(Scalar::Str("alice@example.com".into())),
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
                    }),
                },
                Field {
                    name: "settings".into(),
                    value: Value::Association(vec![(
                        Value::Scalar(Scalar::Sym("theme".into())),
                        Value::Scalar(Scalar::Str("dark".into())),
                    )]),
                },
            ],
        })
    }

    #[test]
    fn query_nested_field() {
        let root = sample_root();
        let result = query(&root, ".user.email").unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(
            &result[0],
            Value::Scalar(Scalar::Str(v)) if v == "alice@example.com"
        ));
    }

    #[test]
    fn query_flatten_and_index() {
        let root = sample_root();
        let all_tags = query(&root, ".user.tags[]").unwrap();
        assert_eq!(all_tags.len(), 2);

        let last_tag = query(&root, ".user.tags[-1]").unwrap();
        assert_eq!(last_tag.len(), 1);
        assert!(matches!(&last_tag[0], Value::Scalar(Scalar::Str(v)) if v == "ops"));
    }

    #[test]
    fn query_map_key() {
        let root = sample_root();
        let result = query(&root, ".settings['theme]").unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], Value::Scalar(Scalar::Str(v)) if v == "dark"));
    }
}
