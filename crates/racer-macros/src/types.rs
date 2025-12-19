use proc_macro2::TokenStream;
use quote::quote;

pub fn parse_type(type_str: &str) -> TokenStream {
    match type_str.trim() {
        "u8" => quote! { u8 },
        "u16" => quote! { u16 },
        "u32" => quote! { u32 },
        "u64" => quote! { u64 },
        "i8" => quote! { i8 },
        "i16" => quote! { i16 },
        "i32" => quote! { i32 },
        "i64" => quote! { i64 },
        "f32" => quote! { f32 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        "string" => quote! { String },
        "bytes" => quote! { Vec<u8> },

        s if s.starts_with("array<") && s.ends_with('>') => {
            let inner = &s[6..s.len() - 1];
            let inner_type = parse_type(inner);
            quote! { Vec<#inner_type> }
        }

        s if s.starts_with("map<") && s.ends_with('>') => {
            let inner = &s[4..s.len() - 1];
            if let Some((key, value)) = inner.split_once(',') {
                let key_type = parse_type(key.trim());
                let value_type = parse_type(value.trim());
                quote! { std::collections::HashMap<#key_type, #value_type> }
            } else {
                quote! { std::collections::HashMap<String, String> }
            }
        }

        other => {
            let ident = syn::Ident::new(other, proc_macro2::Span::call_site());
            quote! { #ident }
        }
    }
}

pub fn is_numeric_type(type_str: &str) -> bool {
    matches!(
        type_str,
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64"
    )
}

pub fn supports_length_validation(type_str: &str) -> bool {
    type_str == "string" || type_str == "bytes" || type_str.starts_with("array<")
}

pub fn default_value(type_str: &str) -> TokenStream {
    match type_str.trim() {
        "string" => quote! { String::new() },
        "bytes" => quote! { Vec::new() },
        "bool" => quote! { false },
        s if s.starts_with("array<") => quote! { Vec::new() },
        s if s.starts_with("map<") => quote! { std::collections::HashMap::new() },
        _ => quote! { Default::default() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8_should_map_to_u8() {
        let tokens = parse_type("u8");
        assert_eq!(tokens.to_string(), "u8");
    }

    #[test]
    fn u16_should_map_to_u16() {
        let tokens = parse_type("u16");
        assert_eq!(tokens.to_string(), "u16");
    }

    #[test]
    fn u32_should_map_to_u32() {
        let tokens = parse_type("u32");
        assert_eq!(tokens.to_string(), "u32");
    }

    #[test]
    fn u64_should_map_to_u64() {
        let tokens = parse_type("u64");
        assert_eq!(tokens.to_string(), "u64");
    }

    #[test]
    fn i8_should_map_to_i8() {
        let tokens = parse_type("i8");
        assert_eq!(tokens.to_string(), "i8");
    }

    #[test]
    fn i16_should_map_to_i16() {
        let tokens = parse_type("i16");
        assert_eq!(tokens.to_string(), "i16");
    }

    #[test]
    fn i32_should_map_to_i32() {
        let tokens = parse_type("i32");
        assert_eq!(tokens.to_string(), "i32");
    }

    #[test]
    fn i64_should_map_to_i64() {
        let tokens = parse_type("i64");
        assert_eq!(tokens.to_string(), "i64");
    }

    #[test]
    fn f32_should_map_to_f32() {
        let tokens = parse_type("f32");
        assert_eq!(tokens.to_string(), "f32");
    }

    #[test]
    fn f64_should_map_to_f64() {
        let tokens = parse_type("f64");
        assert_eq!(tokens.to_string(), "f64");
    }

    #[test]
    fn bool_should_map_to_bool() {
        let tokens = parse_type("bool");
        assert_eq!(tokens.to_string(), "bool");
    }

    #[test]
    fn string_should_map_to_string() {
        let tokens = parse_type("string");
        assert_eq!(tokens.to_string(), "String");
    }

    #[test]
    fn bytes_should_map_to_vec_u8() {
        let tokens = parse_type("bytes");
        assert_eq!(tokens.to_string(), "Vec < u8 >");
    }

    #[test]
    fn array_of_u64_should_map_to_vec_u64() {
        let tokens = parse_type("array<u64>");
        assert_eq!(tokens.to_string(), "Vec < u64 >");
    }

    #[test]
    fn array_of_f64_should_map_to_vec_f64() {
        let tokens = parse_type("array<f64>");
        assert_eq!(tokens.to_string(), "Vec < f64 >");
    }

    #[test]
    fn array_of_string_should_map_to_vec_string() {
        let tokens = parse_type("array<string>");
        assert_eq!(tokens.to_string(), "Vec < String >");
    }

    #[test]
    fn array_of_bool_should_map_to_vec_bool() {
        let tokens = parse_type("array<bool>");
        assert_eq!(tokens.to_string(), "Vec < bool >");
    }

    #[test]
    fn array_of_bytes_should_map_to_vec_vec_u8() {
        let tokens = parse_type("array<bytes>");
        assert_eq!(tokens.to_string(), "Vec < Vec < u8 > >");
    }

    #[test]
    fn array_of_u8_should_map_to_vec_u8() {
        let tokens = parse_type("array<u8>");
        assert_eq!(tokens.to_string(), "Vec < u8 >");
    }

    #[test]
    fn array_of_i32_should_map_to_vec_i32() {
        let tokens = parse_type("array<i32>");
        assert_eq!(tokens.to_string(), "Vec < i32 >");
    }

    #[test]
    fn map_string_string_should_map_to_hashmap() {
        let tokens = parse_type("map<string, string>");
        assert_eq!(tokens.to_string(), "std :: collections :: HashMap < String , String >");
    }

    #[test]
    fn map_string_u64_should_map_to_hashmap() {
        let tokens = parse_type("map<string, u64>");
        assert_eq!(tokens.to_string(), "std :: collections :: HashMap < String , u64 >");
    }

    #[test]
    fn map_u64_string_should_map_to_hashmap() {
        let tokens = parse_type("map<u64, string>");
        assert_eq!(tokens.to_string(), "std :: collections :: HashMap < u64 , String >");
    }

    #[test]
    fn map_string_f64_should_map_to_hashmap() {
        let tokens = parse_type("map<string, f64>");
        assert_eq!(tokens.to_string(), "std :: collections :: HashMap < String , f64 >");
    }

    #[test]
    fn map_with_spaces_should_still_parse() {
        let tokens = parse_type("map<string,   u64>");
        assert_eq!(tokens.to_string(), "std :: collections :: HashMap < String , u64 >");
    }

    #[test]
    fn map_string_bool_should_map_to_hashmap() {
        let tokens = parse_type("map<string, bool>");
        assert_eq!(tokens.to_string(), "std :: collections :: HashMap < String , bool >");
    }

    #[test]
    fn should_trim_whitespace_from_type_string() {
        let tokens = parse_type("  u64  ");
        assert_eq!(tokens.to_string(), "u64");
    }

    #[test]
    fn unknown_type_should_be_used_as_identifier() {
        let tokens = parse_type("CustomType");
        assert_eq!(tokens.to_string(), "CustomType");
    }

    #[test]
    fn malformed_map_without_comma_should_fallback_to_string_string() {
        let tokens = parse_type("map<string string>");
        assert_eq!(tokens.to_string(), "std :: collections :: HashMap < String , String >");
    }

    #[test]
    fn u8_should_be_numeric() {
        assert!(is_numeric_type("u8"));
    }

    #[test]
    fn u16_should_be_numeric() {
        assert!(is_numeric_type("u16"));
    }

    #[test]
    fn u32_should_be_numeric() {
        assert!(is_numeric_type("u32"));
    }

    #[test]
    fn u64_should_be_numeric() {
        assert!(is_numeric_type("u64"));
    }

    #[test]
    fn i8_should_be_numeric() {
        assert!(is_numeric_type("i8"));
    }

    #[test]
    fn i16_should_be_numeric() {
        assert!(is_numeric_type("i16"));
    }

    #[test]
    fn i32_should_be_numeric() {
        assert!(is_numeric_type("i32"));
    }

    #[test]
    fn i64_should_be_numeric() {
        assert!(is_numeric_type("i64"));
    }

    #[test]
    fn f32_should_be_numeric() {
        assert!(is_numeric_type("f32"));
    }

    #[test]
    fn f64_should_be_numeric() {
        assert!(is_numeric_type("f64"));
    }

    #[test]
    fn string_should_not_be_numeric() {
        assert!(!is_numeric_type("string"));
    }

    #[test]
    fn bool_should_not_be_numeric() {
        assert!(!is_numeric_type("bool"));
    }

    #[test]
    fn bytes_should_not_be_numeric() {
        assert!(!is_numeric_type("bytes"));
    }

    #[test]
    fn array_of_u64_should_not_be_numeric() {
        assert!(!is_numeric_type("array<u64>"));
    }

    #[test]
    fn map_should_not_be_numeric() {
        assert!(!is_numeric_type("map<string, u64>"));
    }

    #[test]
    fn custom_type_should_not_be_numeric() {
        assert!(!is_numeric_type("CustomType"));
    }

    #[test]
    fn string_should_support_length_validation() {
        assert!(supports_length_validation("string"));
    }

    #[test]
    fn bytes_should_support_length_validation() {
        assert!(supports_length_validation("bytes"));
    }

    #[test]
    fn array_of_u64_should_support_length_validation() {
        assert!(supports_length_validation("array<u64>"));
    }

    #[test]
    fn array_of_string_should_support_length_validation() {
        assert!(supports_length_validation("array<string>"));
    }

    #[test]
    fn array_of_any_type_should_support_length_validation() {
        assert!(supports_length_validation("array<CustomType>"));
    }

    #[test]
    fn u64_should_not_support_length_validation() {
        assert!(!supports_length_validation("u64"));
    }

    #[test]
    fn f64_should_not_support_length_validation() {
        assert!(!supports_length_validation("f64"));
    }

    #[test]
    fn bool_should_not_support_length_validation() {
        assert!(!supports_length_validation("bool"));
    }

    #[test]
    fn map_should_not_support_length_validation() {
        assert!(!supports_length_validation("map<string, string>"));
    }

    #[test]
    fn i32_should_not_support_length_validation() {
        assert!(!supports_length_validation("i32"));
    }

    #[test]
    fn string_default_should_be_string_new() {
        let tokens = default_value("string");
        assert_eq!(tokens.to_string(), "String :: new ()");
    }

    #[test]
    fn bytes_default_should_be_vec_new() {
        let tokens = default_value("bytes");
        assert_eq!(tokens.to_string(), "Vec :: new ()");
    }

    #[test]
    fn bool_default_should_be_false() {
        let tokens = default_value("bool");
        assert_eq!(tokens.to_string(), "false");
    }

    #[test]
    fn array_default_should_be_vec_new() {
        let tokens = default_value("array<u64>");
        assert_eq!(tokens.to_string(), "Vec :: new ()");
    }

    #[test]
    fn map_default_should_be_hashmap_new() {
        let tokens = default_value("map<string, u64>");
        assert_eq!(tokens.to_string(), "std :: collections :: HashMap :: new ()");
    }

    #[test]
    fn u64_default_should_be_default_default() {
        let tokens = default_value("u64");
        assert_eq!(tokens.to_string(), "Default :: default ()");
    }

    #[test]
    fn f64_default_should_be_default_default() {
        let tokens = default_value("f64");
        assert_eq!(tokens.to_string(), "Default :: default ()");
    }

    #[test]
    fn custom_type_default_should_be_default_default() {
        let tokens = default_value("CustomType");
        assert_eq!(tokens.to_string(), "Default :: default ()");
    }

    #[test]
    fn array_of_string_default_should_be_vec_new() {
        let tokens = default_value("array<string>");
        assert_eq!(tokens.to_string(), "Vec :: new ()");
    }
}

