//! # surp-derive
//!
//! Proc-macro crate providing `#[derive(Surp)]` and `#[derive(SurpSchema)]`
//! for automatic serialization with stable field IDs.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use surp_derive::{Surp, SurpSchema};
//! use surp_core::Value;
//!
//! #[derive(Debug, PartialEq, Surp, SurpSchema)]
//! struct Person {
//!     #[surp(id = 1)] name: String,
//!     #[surp(id = 2)] age: u8,
//!     #[surp(id = 3)] tags: Vec<String>,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Lit, parse_macro_input};

/// Resolve the numeric `#[surp(id = N)]` for a field, falling back to a
/// deterministic hash of the field name when no explicit id is given.
///
/// This is shared by `derive_surp` and `derive_surp_schema` so that the id a
/// struct's wire encoding actually uses (folded into `schema_fingerprint`,
/// and now used directly as the wire object key) always agrees with the id
/// reported by `schema_info()`. Prior to this helper the two macros computed
/// fallback ids differently (`derive_surp` hashed the field name,
/// `derive_surp_schema` always reported `0`), which made `schema_info()`
/// lie about the id actually on the wire whenever an explicit id was
/// omitted.
///
/// Any malformed `#[surp(...)]` attribute (an unknown key such as the typo
/// `idd`, a missing `= value`, a non-integer literal, or an integer literal
/// that doesn't fit in a `u64`) is surfaced as a `syn::Error` instead of
/// being silently swallowed and falling back to the hash-derived id.
fn resolve_field_id(attrs: &[Attribute], field_name: &str) -> syn::Result<u64> {
    let mut field_id: Option<u64> = None;

    for attr in attrs {
        if attr.path().is_ident("surp") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    match lit {
                        Lit::Int(lit_int) => {
                            let parsed: u64 = lit_int.base10_parse().map_err(|e| {
                                syn::Error::new_spanned(
                                    &lit_int,
                                    format!(
                                        "invalid Surp field id `{}`: {e}",
                                        lit_int.base10_digits()
                                    ),
                                )
                            })?;
                            field_id = Some(parsed);
                            Ok(())
                        }
                        other => Err(syn::Error::new_spanned(
                            other,
                            "Surp field id must be an unsigned integer literal, e.g. `#[surp(id = 1)]`",
                        )),
                    }
                } else {
                    Err(meta.error(
                        "unknown key in `#[surp(...)]` attribute; expected `id = N`",
                    ))
                }
            })?;
        }
    }

    Ok(field_id.unwrap_or_else(|| {
        // If no explicit id, use hash of field name as fallback.
        // This is not recommended but provides a default.
        let hash = xxhash_rust::xxh64::xxh64(field_name.as_bytes(), 0);
        hash & 0xFFFF // Use lower 16 bits.
    }))
}

/// Check a struct's resolved field ids (explicit or hash-fallback) for
/// duplicates. Two fields sharing the same wire id would silently corrupt
/// the "stable field id" guarantee: they'd collide on the same wire key and
/// on the same contribution to `schema_fingerprint`/`schema_info()`.
///
/// `fields` is `(field_ident, field_name, resolved_id)`; the error is
/// spanned at the *second* field carrying a given id (i.e. the field whose
/// id first collides with an earlier one), so the diagnostic points at the
/// offending duplicate rather than the whole struct.
fn check_duplicate_ids(fields: &[(&syn::Ident, &str, u64)]) -> syn::Result<()> {
    for (i, (ident, _name, id)) in fields.iter().enumerate() {
        for (other_name, other_id) in fields[..i].iter().map(|(_, n, id)| (*n, *id)) {
            if *id == other_id {
                return Err(syn::Error::new_spanned(
                    ident,
                    format!("duplicate Surp field id {id} (also used by field `{other_name}`)"),
                ));
            }
        }
    }
    Ok(())
}

/// Derive the `Surp` trait for a struct, generating encode/decode
/// implementations with stable field IDs.
///
/// Each field must be annotated with `#[surp(id = N)]` where N is a
/// unique, stable integer identifier for schema evolution.
#[proc_macro_derive(Surp, attributes(surp))]
pub fn derive_surp(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(_) | Fields::Unit => {
                return syn::Error::new_spanned(
                    &input.ident,
                    "Surp derive only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            return syn::Error::new_spanned(&input.ident, "Surp derive only supports structs")
                .to_compile_error()
                .into();
        }
    };

    // Extract field names and their surp(id = N) attributes.
    let mut field_infos = Vec::new();
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let id = match resolve_field_id(&field.attrs, &field_name_str) {
            Ok(id) => id,
            Err(err) => return err.to_compile_error().into(),
        };

        field_infos.push((field_name.clone(), field_name_str, id));
    }

    if let Err(err) = check_duplicate_ids(
        &field_infos
            .iter()
            .map(|(ident, name, id)| (ident, name.as_str(), *id))
            .collect::<Vec<_>>(),
    ) {
        return err.to_compile_error().into();
    }

    // Generate schema fingerprint from type name + field IDs.
    let schema_str = format!(
        "{}:{}",
        name_str,
        field_infos
            .iter()
            .map(|(_, n, id)| format!("{n}={id}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let fingerprint = xxhash_rust::xxh64::xxh64(schema_str.as_bytes(), 0);

    // Generate to_surp_value: creates an Object keyed by the stringified
    // numeric field id (not the field name). This is what actually makes
    // `#[surp(id = N)]` "stable": renaming a Rust field while keeping the
    // same id produces byte-identical wire output, and the wire no longer
    // repeats full field-name strings for every encoded value.
    let encode_fields: Vec<_> = field_infos
        .iter()
        .map(|(fname, _fname_str, id)| {
            let id_str = id.to_string();
            quote! {
                (
                    #id_str.to_string(),
                    surp_core::Surp::to_surp_value(&self.#fname)
                )
            }
        })
        .collect();

    // Generate from_surp_value: matches field ids (as strings) from Object
    // entries, since the wire key is now the stringified id.
    let decode_fields_init: Vec<_> = field_infos
        .iter()
        .map(|(fname, _fname_str, _id)| {
            quote! {
                let mut #fname = None;
            }
        })
        .collect();

    let decode_fields_match: Vec<_> = field_infos
        .iter()
        .map(|(fname, _fname_str, id)| {
            let id_str = id.to_string();
            quote! {
                #id_str => {
                    #fname = Some(surp_core::Surp::from_surp_value(v)?);
                }
            }
        })
        .collect();

    let decode_fields_unwrap: Vec<_> = field_infos
        .iter()
        .map(|(fname, fname_str, _id)| {
            quote! {
                #fname: #fname.ok_or_else(|| surp_core::SurpError::SchemaMismatch(
                    format!("missing field '{}' in {}", #fname_str, #name_str)
                ))?
            }
        })
        .collect();

    let expanded = quote! {
        impl surp_core::Surp for #name {
            fn to_surp_value(&self) -> surp_core::Value {
                surp_core::Value::Object(vec![
                    #(#encode_fields),*
                ])
            }

            fn from_surp_value(value: &surp_core::Value) -> surp_core::Result<Self> {
                match value {
                    surp_core::Value::Object(entries) => {
                        #(#decode_fields_init)*

                        for (k, v) in entries {
                            match k.as_str() {
                                #(#decode_fields_match)*
                                _ => {} // Skip unknown fields for forward compatibility.
                            }
                        }

                        Ok(Self {
                            #(#decode_fields_unwrap),*
                        })
                    }
                    _ => Err(surp_core::SurpError::SchemaMismatch(
                        format!("expected object for {}", #name_str)
                    )),
                }
            }

            fn schema_fingerprint() -> u64 {
                #fingerprint
            }

            fn type_name() -> &'static str {
                #name_str
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive `SurpSchema` — generates a `schema_info()` method that returns
/// metadata about the struct's field IDs and types.
///
/// This is a companion to `#[derive(Surp)]` and provides introspection.
#[proc_macro_derive(SurpSchema, attributes(surp))]
pub fn derive_surp_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(_) | Fields::Unit => {
                return syn::Error::new_spanned(
                    &input.ident,
                    "SurpSchema only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        Data::Enum(_) | Data::Union(_) => {
            return syn::Error::new_spanned(&input.ident, "SurpSchema only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let mut field_ids = Vec::new();
    let mut field_entries = Vec::new();
    for field in fields {
        let ident = field.ident.as_ref().unwrap();
        let fname = ident.to_string();
        let fid = match resolve_field_id(&field.attrs, &fname) {
            Ok(id) => id,
            Err(err) => return err.to_compile_error().into(),
        };

        field_ids.push((ident.clone(), fname.clone(), fid));
        field_entries.push(quote! {
            (#fname, #fid)
        });
    }

    if let Err(err) = check_duplicate_ids(
        &field_ids
            .iter()
            .map(|(ident, name, id)| (ident, name.as_str(), *id))
            .collect::<Vec<_>>(),
    ) {
        return err.to_compile_error().into();
    }

    let expanded = quote! {
        impl #name {
            /// Returns schema metadata: pairs of (field_name, field_id).
            pub fn schema_info() -> &'static [(&'static str, u64)] {
                &[ #(#field_entries),* ]
            }

            /// Returns the type name.
            pub fn schema_type_name() -> &'static str {
                #name_str
            }
        }
    };

    TokenStream::from(expanded)
}
