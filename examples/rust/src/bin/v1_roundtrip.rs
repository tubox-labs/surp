use surp_core::{Decoder, Encoder, Value};

fn main() -> surp_core::Result<()> {
    let value = Value::Object(vec![
        ("id".into(), Value::UInt(1001)),
        ("name".into(), Value::Str("Alice".into())),
        ("active".into(), Value::Bool(true)),
        (
            "tags".into(),
            Value::Array(vec![Value::Str("admin".into()), Value::Str("ops".into())]),
        ),
        ("avatar".into(), Value::Bytes(vec![1, 2, 3])),
    ]);

    let mut encoder = Encoder::new();
    encoder.enable_dedup();
    encoder.encode_value(&value)?;
    let bytes = encoder.finish()?;

    let mut decoder = Decoder::new(&bytes);
    let values = decoder.decode_all_owned()?;
    assert_eq!(values, vec![value]);

    println!("encoded {} byte v1 Surp document", bytes.len());
    Ok(())
}
