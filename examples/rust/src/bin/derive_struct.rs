use surp_core::{Surp, SurpBytes};

#[derive(Debug, PartialEq, surp_derive::Surp, surp_derive::SurpSchema)]
struct Profile {
    #[surp(id = 1)]
    name: String,
    #[surp(id = 2)]
    age: u8,
    #[surp(id = 3)]
    tags: Vec<String>,
    #[surp(id = 4)]
    avatar: SurpBytes,
}

fn main() -> surp_core::Result<()> {
    let profile = Profile {
        name: "Alice".into(),
        age: 30,
        tags: vec!["admin".into(), "ops".into()],
        avatar: SurpBytes::new(vec![1, 2, 3]),
    };

    let bytes = profile.to_surp_bytes()?;
    let decoded = Profile::from_surp_bytes(&bytes)?;
    assert_eq!(decoded, profile);

    println!(
        "{} schema: {:?}",
        Profile::schema_type_name(),
        Profile::schema_info()
    );
    println!("encoded {} byte derived profile", bytes.len());
    Ok(())
}
