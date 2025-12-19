use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MessageConfig {
    pub message: MessageDef,
}

#[derive(Debug, Deserialize)]
pub struct MessageDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub id_field: bool,
    #[serde(default)]
    pub required: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
}

impl FieldDef {
    pub fn has_validation(&self) -> bool {
        self.required
            || self.min.is_some()
            || self.max.is_some()
            || self.min_length.is_some()
            || self.max_length.is_some()
    }
}

pub fn parse_toml(content: &str) -> Result<MessageConfig, toml::de::Error> {
    toml::from_str(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_minimal_valid_toml() {
        let toml = r#"
            [message]
            name = "TestMessage"
            [[message.fields]]
            name = "id"
            type = "u64"
        "#;
        
        let result = parse_toml(toml);
        assert!(result.is_ok(), "minimal valid TOML should parse successfully");
    }

    #[test]
    fn should_extract_message_name() {
        let toml = r#"
            [message]
            name = "MyCustomMessage"
            [[message.fields]]
            name = "field"
            type = "u64"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.name, "MyCustomMessage");
    }

    #[test]
    fn should_fail_on_missing_message_section() {
        let toml = r#"
            [[fields]]
            name = "id"
            type = "u64"
        "#;
        
        let result = parse_toml(toml);
        assert!(result.is_err(), "missing [message] section should fail");
    }

    #[test]
    fn should_fail_on_invalid_toml_syntax() {
        let toml = "this is not valid toml {{{";
        
        let result = parse_toml(toml);
        assert!(result.is_err(), "invalid TOML syntax should fail");
    }

    #[test]
    fn should_handle_unicode_in_message_name() {
        let toml = r#"
            [message]
            name = "消息"
            [[message.fields]]
            name = "id"
            type = "u64"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.name, "消息");
    }

    #[test]
    fn should_parse_field_name() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "my_field_name"
            type = "u64"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].name, "my_field_name");
    }

    #[test]
    fn should_fail_on_missing_field_name() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            type = "u64"
        "#;
        
        let result = parse_toml(toml);
        assert!(result.is_err(), "missing field name should fail");
    }

    #[test]
    fn should_parse_multiple_fields() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "field1"
            type = "u64"
            [[message.fields]]
            name = "field2"
            type = "string"
            [[message.fields]]
            name = "field3"
            type = "bool"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields.len(), 3);
        assert_eq!(config.message.fields[0].name, "field1");
        assert_eq!(config.message.fields[1].name, "field2");
        assert_eq!(config.message.fields[2].name, "field3");
    }

    #[test]
    fn should_preserve_field_order() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "zebra"
            type = "u64"
            [[message.fields]]
            name = "alpha"
            type = "u64"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].name, "zebra", "fields should maintain definition order");
        assert_eq!(config.message.fields[1].name, "alpha");
    }

    #[test]
    fn id_field_should_default_to_false() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "id"
            type = "u64"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert!(!config.message.fields[0].id_field, "id_field should default to false");
    }

    #[test]
    fn id_field_should_be_settable_to_true() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "id"
            type = "u64"
            id_field = true
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert!(config.message.fields[0].id_field);
    }

    #[test]
    fn required_should_default_to_false() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "string"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert!(!config.message.fields[0].required);
    }

    #[test]
    fn required_should_be_settable_to_true() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "string"
            required = true
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert!(config.message.fields[0].required);
    }

    #[test]
    fn min_should_default_to_none() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "f64"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert!(config.message.fields[0].min.is_none());
    }

    #[test]
    fn min_should_parse_positive_value() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "f64"
            min = 10.5
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].min, Some(10.5));
    }

    #[test]
    fn min_should_parse_negative_value() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "f64"
            min = -100.0
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].min, Some(-100.0));
    }

    #[test]
    fn max_should_default_to_none() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "f64"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert!(config.message.fields[0].max.is_none());
    }

    #[test]
    fn max_should_parse_value() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "f64"
            max = 999.99
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].max, Some(999.99));
    }

    #[test]
    fn min_length_should_default_to_none() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "string"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert!(config.message.fields[0].min_length.is_none());
    }

    #[test]
    fn min_length_should_parse_value() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "string"
            min_length = 5
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].min_length, Some(5));
    }

    #[test]
    fn max_length_should_default_to_none() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "string"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert!(config.message.fields[0].max_length.is_none());
    }

    #[test]
    fn max_length_should_parse_value() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "string"
            max_length = 100
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].max_length, Some(100));
    }

    #[test]
    fn should_parse_all_attributes_together() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "value"
            type = "f64"
            id_field = true
            required = true
            min = 0.0
            max = 100.0
            min_length = 1
            max_length = 10
        "#;
        
        let config = parse_toml(toml).unwrap();
        let field = &config.message.fields[0];
        assert!(field.id_field);
        assert!(field.required);
        assert_eq!(field.min, Some(0.0));
        assert_eq!(field.max, Some(100.0));
        assert_eq!(field.min_length, Some(1));
        assert_eq!(field.max_length, Some(10));
    }

    fn make_field(
        required: bool,
        min: Option<f64>,
        max: Option<f64>,
        min_length: Option<usize>,
        max_length: Option<usize>,
    ) -> FieldDef {
        FieldDef {
            name: "test".into(),
            field_type: "u64".into(),
            id_field: false,
            required,
            min,
            max,
            min_length,
            max_length,
        }
    }

    #[test]
    fn has_validation_should_return_false_when_no_validation_set() {
        let field = make_field(false, None, None, None, None);
        assert!(!field.has_validation());
    }

    #[test]
    fn has_validation_should_return_true_when_required_is_true() {
        let field = make_field(true, None, None, None, None);
        assert!(field.has_validation());
    }

    #[test]
    fn has_validation_should_return_true_when_min_is_set() {
        let field = make_field(false, Some(0.0), None, None, None);
        assert!(field.has_validation());
    }

    #[test]
    fn has_validation_should_return_true_when_max_is_set() {
        let field = make_field(false, None, Some(100.0), None, None);
        assert!(field.has_validation());
    }

    #[test]
    fn has_validation_should_return_true_when_min_length_is_set() {
        let field = make_field(false, None, None, Some(1), None);
        assert!(field.has_validation());
    }

    #[test]
    fn has_validation_should_return_true_when_max_length_is_set() {
        let field = make_field(false, None, None, None, Some(100));
        assert!(field.has_validation());
    }

    #[test]
    fn should_parse_array_of_primitives() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "values"
            type = "array<f64>"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].field_type, "array<f64>");
    }

    #[test]
    fn should_parse_map_type() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "metadata"
            type = "map<string, string>"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].field_type, "map<string, string>");
    }

    #[test]
    fn should_parse_bytes_type() {
        let toml = r#"
            [message]
            name = "Test"
            [[message.fields]]
            name = "data"
            type = "bytes"
        "#;
        
        let config = parse_toml(toml).unwrap();
        assert_eq!(config.message.fields[0].field_type, "bytes");
    }
}

