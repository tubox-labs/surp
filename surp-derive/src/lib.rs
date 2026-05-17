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
use syn::{Data, DeriveInput, Fields, Lit, parse_macro_input};

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
            _ => panic!("Surp derive only supports structs with named fields"),
        },
        _ => panic!("Surp derive only supports structs"),
    };

    // Extract field names and their surp(id = N) attributes.
    let mut field_infos = Vec::new();
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let mut field_id: Option<u64> = None;

        for attr in &field.attrs {
            if attr.path().is_ident("surp") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("id") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Int(lit_int) = lit {
                            field_id = Some(lit_int.base10_parse().unwrap());
                        }
                    }
                    Ok(())
                })
                .ok();
            }
        }

        let id = field_id.unwrap_or_else(|| {
            // If no explicit id, use hash of field name as fallback.
            // This is not recommended but provides a default.
            let hash = xxhash_rust::xxh64::xxh64(field_name_str.as_bytes(), 0);
            hash & 0xFFFF // Use lower 16 bits.
        });

        field_infos.push((field_name.clone(), field_name_str, id));
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

    // Generate to_surp_value: creates an Object with field name keys.
    let encode_fields: Vec<_> = field_infos
        .iter()
        .map(|(fname, fname_str, _id)| {
            quote! {
                (
                    #fname_str.to_string(),
                    surp_core::Surp::to_surp_value(&self.#fname)
                )
            }
        })
        .collect();

    // Generate from_surp_value: matches field names from Object entries.
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
        .map(|(fname, fname_str, _id)| {
            quote! {
                #fname_str => {
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
            _ => panic!("SurpSchema only supports structs with named fields"),
        },
        _ => panic!("SurpSchema only supports structs"),
    };

    let mut field_entries = Vec::new();
    for field in fields {
        let fname = field.ident.as_ref().unwrap().to_string();
        let mut fid: u64 = 0;

        for attr in &field.attrs {
            if attr.path().is_ident("surp") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("id") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Int(lit_int) = lit {
                            fid = lit_int.base10_parse().unwrap();
                        }
                    }
                    Ok(())
                })
                .ok();
            }
        }

        field_entries.push(quote! {
            (#fname, #fid)
        });
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
