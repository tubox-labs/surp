use surp_core::Value;
use surp_core::text::{parse, pretty_print};

const TEXT: &str = r#"
{
  id: 1001;
  name: "Alice";
  active: true;
  tags: ["admin", "ops"];
  avatar: b64#AQID;
}
"#;

fn main() -> surp_core::Result<()> {
    let value = parse(TEXT)?;
    let expected = Value::Object(vec![
        ("id".into(), Value::UInt(1001)),
        ("name".into(), Value::Str("Alice".into())),
        ("active".into(), Value::Bool(true)),
        (
            "tags".into(),
            Value::Array(vec![Value::Str("admin".into()), Value::Str("ops".into())]),
        ),
        ("avatar".into(), Value::Bytes(vec![1, 2, 3])),
    ]);
    assert_eq!(value, expected);

    let rendered = pretty_print(&value, 2);
    assert!(rendered.contains("name: \"Alice\";"));
    println!("{rendered}");
    Ok(())
}
