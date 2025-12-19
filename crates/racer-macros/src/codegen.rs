use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemStruct, LitStr};

use crate::parser::{self, FieldDef, MessageConfig};
use crate::types;

pub fn generate(path_lit: &LitStr, input: &ItemStruct) -> Result<TokenStream, syn::Error> {
    let toml_path = path_lit.value();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        syn::Error::new_spanned(path_lit, "CARGO_MANIFEST_DIR not set")
    })?;

    let full_path = std::path::Path::new(&manifest_dir).join(&toml_path);

    let toml_content = std::fs::read_to_string(&full_path).map_err(|e| {
        syn::Error::new_spanned(
            path_lit,
            format!("failed to read '{}': {}", full_path.display(), e),
        )
    })?;

    let config: MessageConfig = parser::parse_toml(&toml_content).map_err(|e| {
        syn::Error::new_spanned(path_lit, format!("invalid TOML: {}", e))
    })?;

    let struct_name = &input.ident;
    if config.message.name != struct_name.to_string() {
        return Err(syn::Error::new_spanned(
            struct_name,
            format!(
                "struct name '{}' does not match TOML name '{}'",
                struct_name, config.message.name
            ),
        ));
    }

    let fields = generate_fields(&config.message.fields);

    let id_field = config
        .message
        .fields
        .iter()
        .find(|f| f.id_field)
        .map(|f| format_ident!("{}", f.name));

    let id_impl = if let Some(id_field) = id_field {
        quote! { self.#id_field }
    } else {
        let first_u64 = config
            .message
            .fields
            .iter()
            .find(|f| f.field_type == "u64")
            .map(|f| format_ident!("{}", f.name));

        if let Some(field) = first_u64 {
            quote! { self.#field }
        } else {
            quote! { 0 }
        }
    };

    let validation = generate_validation(&config.message.fields);

    let vis = &input.vis;
    let attrs = &input.attrs;

    Ok(quote! {
        #(#attrs)*
        #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
        #vis struct #struct_name {
            #fields
        }

        impl racer_core::Message for #struct_name {
            fn id(&self) -> u64 {
                #id_impl
            }

            fn validate(&self) -> racer_core::ValidationResult {
                use racer_core::FieldValidator;
                #validation
                Ok(())
            }
        }
    })
}

fn generate_fields(fields: &[FieldDef]) -> TokenStream {
    let field_tokens: Vec<_> = fields
        .iter()
        .map(|field| {
            let name = format_ident!("{}", field.name);
            let ty = types::parse_type(&field.field_type);
            quote! {
                pub #name: #ty,
            }
        })
        .collect();

    quote! { #(#field_tokens)* }
}

fn generate_validation(fields: &[FieldDef]) -> TokenStream {
    let validations: Vec<_> = fields
        .iter()
        .filter(|f| f.has_validation())
        .map(|field| generate_field_validation(field))
        .collect();

    quote! { #(#validations)* }
}

fn generate_field_validation(field: &FieldDef) -> TokenStream {
    let name = format_ident!("{}", field.name);
    let name_str = &field.name;
    let mut checks = Vec::new();

    // Required check
    if field.required {
        checks.push(quote! {
            if self.#name.is_empty() {
                return Err(racer_core::ValidationError::required(#name_str));
            }
        });
    }

    if types::is_numeric_type(&field.field_type) {
        if let Some(min) = field.min {
            checks.push(quote! {
                if (self.#name as f64) < #min {
                    return Err(racer_core::ValidationError::min_value(
                        #name_str,
                        #min,
                        self.#name as f64,
                    ));
                }
            });
        }

        if let Some(max) = field.max {
            checks.push(quote! {
                if (self.#name as f64) > #max {
                    return Err(racer_core::ValidationError::max_value(
                        #name_str,
                        #max,
                        self.#name as f64,
                    ));
                }
            });
        }
    }

    if types::supports_length_validation(&field.field_type) {
        if let Some(min_len) = field.min_length {
            checks.push(quote! {
                if self.#name.len() < #min_len {
                    return Err(racer_core::ValidationError::min_length(
                        #name_str,
                        #min_len,
                        self.#name.len(),
                    ));
                }
            });
        }

        if let Some(max_len) = field.max_length {
            checks.push(quote! {
                if self.#name.len() > #max_len {
                    return Err(racer_core::ValidationError::max_length(
                        #name_str,
                        #max_len,
                        self.#name.len(),
                    ));
                }
            });
        }
    }

    quote! { #(#checks)* }
}
