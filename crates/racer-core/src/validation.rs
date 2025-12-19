use std::fmt;

pub type ValidationResult = Result<(), ValidationError>;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub kind: ValidationKind,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>, kind: ValidationKind) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            kind,
        }
    }

    pub fn required(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(&field, format!("field '{}' is required", field), ValidationKind::Required)
    }

    pub fn min_value(field: impl Into<String>, min: f64, actual: f64) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("field '{}' must be >= {} (got {})", field, min, actual),
            ValidationKind::MinValue { min, actual },
        )
    }

    pub fn max_value(field: impl Into<String>, max: f64, actual: f64) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("field '{}' must be <= {} (got {})", field, max, actual),
            ValidationKind::MaxValue { max, actual },
        )
    }

    pub fn min_length(field: impl Into<String>, min: usize, actual: usize) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("field '{}' must have length >= {} (got {})", field, min, actual),
            ValidationKind::MinLength { min, actual },
        )
    }

    pub fn max_length(field: impl Into<String>, max: usize, actual: usize) -> Self {
        let field = field.into();
        Self::new(
            &field,
            format!("field '{}' must have length <= {} (got {})", field, max, actual),
            ValidationKind::MaxLength { max, actual },
        )
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationKind {
    Required,
    MinValue { min: f64, actual: f64 },
    MaxValue { max: f64, actual: f64 },
    MinLength { min: usize, actual: usize },
    MaxLength { max: usize, actual: usize },
}

pub trait FieldValidator {
    fn is_empty(&self) -> bool;
}

impl FieldValidator for String {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> FieldValidator for Vec<T> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<K, V> FieldValidator for std::collections::HashMap<K, V> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

macro_rules! impl_field_validator_numeric {
    ($($t:ty),*) => {
        $(
            impl FieldValidator for $t {
                fn is_empty(&self) -> bool {
                    false
                }
            }
        )*
    };
}

impl_field_validator_numeric!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, bool);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::required("timestamp");
        assert!(err.to_string().contains("timestamp"));
        assert!(err.to_string().contains("required"));
    }

    #[test]
    fn test_field_validator() {
        assert!(String::new().is_empty());
        assert!(!String::from("hello").is_empty());
        assert!(Vec::<u8>::new().is_empty());
        assert!(!42u64.is_empty());
    }
}
