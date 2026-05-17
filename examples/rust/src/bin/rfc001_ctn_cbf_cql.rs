use surp_core::rfc001;

const CTN: &str = r#"
@surp v1
@encoding cbf

let alice = User
  id = uid"550e8400-e29b-41d4-a716-446655440000"
  name = "Alice"
  role = 'Admin
  tags = ["admin", "ops"]
  settings = map<str, str> ["theme" => "dark", 'region => "us"]
  matrix = tensor<f32>[2, 2]
    [1.0f32, 2.0f32]
    [3.0f32, 4.0f32]

&alice
"#;

fn main() -> surp_core::Result<()> {
    let document = rfc001::parse_document(CTN)?;
    let normalized = rfc001::format_document(&document);
    assert!(normalized.contains("tensor<f32>[2, 2]"));

    let cbf = rfc001::encode_document(
        &document,
        rfc001::EncodeOptions {
            with_symtab: true,
            alignment: 4,
        },
    )?;
    assert_eq!(&cbf[..4], &rfc001::CBF_MAGIC);

    let decoded = rfc001::decode_document(&cbf)?;
    assert_eq!(decoded.header.cbf_version, 1);
    assert_eq!(decoded.header.ctn_version, 1);
    assert_eq!(decoded.header.alignment, 4);
    assert!(decoded.header.has_symtab());

    let root = decoded.document.effective_root()?;
    let name = rfc001::query(&root, ".name")?;
    assert_eq!(rfc001::format_value(&name[0]), "\"Alice\"");

    let tags = rfc001::query(&root, ".tags[]")?;
    assert_eq!(
        tags.iter().map(rfc001::format_value).collect::<Vec<_>>(),
        vec!["\"admin\"".to_string(), "\"ops\"".to_string()]
    );

    println!("encoded {} byte RFC-001 CBF document", cbf.len());
    println!("{}", rfc001::format_document(&decoded.document));
    Ok(())
}
