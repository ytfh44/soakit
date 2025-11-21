/// Utility functions for SoAKit.
///
/// This module provides utility functions for type checking, field name validation,
/// and filtering system fields.
use crate::value::Value;

/// Check if a value is a scalar (rank 0).
///
/// Scalars are single values, not collections. This is a convenience function
/// that calls [`Value::is_scalar`].
///
/// # Arguments
///
/// * `value` - The value to check
///
/// # Returns
///
/// Returns `true` if the value is a scalar, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// use soakit::{is_scalar, Value};
///
/// assert!(is_scalar(&Value::ScalarInt(42)));
/// assert!(is_scalar(&Value::ScalarFloat(3.14)));
/// assert!(!is_scalar(&Value::VectorInt(vec![1, 2, 3])));
/// ```
pub fn is_scalar(value: &Value) -> bool {
    value.is_scalar()
}

/// Check if a value is a vector (rank 1).
///
/// Vectors are 1D arrays of primitive values. This is a convenience function
/// that calls [`Value::is_vector`].
///
/// # Arguments
///
/// * `value` - The value to check
///
/// # Returns
///
/// Returns `true` if the value is a vector, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// use soakit::{is_vector, Value};
///
/// assert!(is_vector(&Value::VectorInt(vec![1, 2, 3])));
/// assert!(is_vector(&Value::VectorFloat(vec![1.0, 2.0])));
/// assert!(!is_vector(&Value::ScalarInt(42)));
/// ```
pub fn is_vector(value: &Value) -> bool {
    value.is_vector()
}

/// Check if a value is a matrix (rank 2+).
///
/// Matrices are nested structures. This is a convenience function that calls
/// [`Value::is_matrix`].
///
/// # Arguments
///
/// * `value` - The value to check
///
/// # Returns
///
/// Returns `true` if the value is a matrix, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// use soakit::{is_matrix, Value};
///
/// let matrix = Value::Matrix(vec![Value::VectorInt(vec![1, 2])]);
/// assert!(is_matrix(&matrix));
/// assert!(!is_matrix(&Value::VectorInt(vec![1, 2])));
/// ```
pub fn is_matrix(value: &Value) -> bool {
    value.is_matrix()
}

/// Validate a field name.
///
/// Field names must not start with an underscore (reserved for system/internal
/// fields) and must not be empty.
///
/// # Arguments
///
/// * `name` - The field name to validate
///
/// # Returns
///
/// Returns `true` if the name is valid, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// use soakit::is_valid_field_name;
///
/// assert!(is_valid_field_name("age"));
/// assert!(is_valid_field_name("field_name"));
/// assert!(!is_valid_field_name("_internal")); // Starts with underscore
/// assert!(!is_valid_field_name("")); // Empty
/// ```
pub fn is_valid_field_name(name: &str) -> bool {
    !name.starts_with('_') && !name.is_empty()
}

/// Filter out system/internal field names (those starting with underscore).
///
/// System fields are those that start with an underscore and are reserved for
/// internal use. This function filters them out from a list of field names.
///
/// # Arguments
///
/// * `names` - A slice of field names to filter
///
/// # Returns
///
/// A vector containing only the non-system field names.
///
/// # Examples
///
/// ```rust
/// use soakit::filter_system_fields;
///
/// let names = vec![
///     "age".to_string(),
///     "_internal".to_string(),
///     "height".to_string(),
///     "_meta".to_string(),
/// ];
/// let filtered = filter_system_fields(&names);
/// assert_eq!(filtered, vec!["age".to_string(), "height".to_string()]);
/// ```
pub fn filter_system_fields(names: &[String]) -> Vec<String> {
    names
        .iter()
        .filter(|n| !n.starts_with('_'))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_type_checking() {
        let scalar = Value::ScalarInt(42);
        let vector = Value::VectorInt(vec![1, 2, 3]);
        let matrix = Value::Matrix(vec![Value::VectorInt(vec![1, 2])]);

        assert!(is_scalar(&scalar));
        assert!(!is_scalar(&vector));
        assert!(!is_scalar(&matrix));

        assert!(!is_vector(&scalar));
        assert!(is_vector(&vector));
        assert!(!is_vector(&matrix));

        assert!(!is_matrix(&scalar));
        assert!(!is_matrix(&vector));
        assert!(is_matrix(&matrix));
    }

    #[test]
    fn test_field_name_validation() {
        assert!(is_valid_field_name("name"));
        assert!(is_valid_field_name("field1"));
        assert!(!is_valid_field_name("_internal"));
        assert!(!is_valid_field_name(""));
    }

    #[test]
    fn test_filter_system_fields() {
        let names = vec![
            "field1".to_string(),
            "_internal".to_string(),
            "field2".to_string(),
            "_meta".to_string(),
        ];
        let filtered = filter_system_fields(&names);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"field1".to_string()));
        assert!(filtered.contains(&"field2".to_string()));
    }

    #[test]
    fn test_type_checking_all_scalar_types() {
        assert!(is_scalar(&Value::ScalarInt(0)));
        assert!(is_scalar(&Value::ScalarFloat(0.0)));
        assert!(is_scalar(&Value::ScalarBool(false)));
        assert!(is_scalar(&Value::ScalarString(String::new())));

        assert!(!is_vector(&Value::ScalarInt(0)));
        assert!(!is_matrix(&Value::ScalarInt(0)));
    }

    #[test]
    fn test_type_checking_all_vector_types() {
        assert!(is_vector(&Value::VectorInt(vec![])));
        assert!(is_vector(&Value::VectorFloat(vec![])));
        assert!(is_vector(&Value::VectorBool(vec![])));
        assert!(is_vector(&Value::VectorString(vec![])));

        assert!(!is_scalar(&Value::VectorInt(vec![])));
        assert!(!is_matrix(&Value::VectorInt(vec![])));
    }

    #[test]
    fn test_type_checking_matrix() {
        let matrix = Value::Matrix(vec![
            Value::VectorInt(vec![1, 2]),
            Value::VectorInt(vec![3, 4]),
        ]);
        assert!(is_matrix(&matrix));
        assert!(!is_scalar(&matrix));
        assert!(!is_vector(&matrix));

        let empty_matrix = Value::Matrix(vec![]);
        assert!(is_matrix(&empty_matrix));
    }

    #[test]
    fn test_field_name_validation_edge_cases() {
        // Valid names
        assert!(is_valid_field_name("a"));
        assert!(is_valid_field_name("field_name"));
        assert!(is_valid_field_name("field-name"));
        assert!(is_valid_field_name("field123"));
        assert!(is_valid_field_name("FieldName"));
        assert!(is_valid_field_name("field_name_123"));

        // Invalid names
        assert!(!is_valid_field_name(""));
        assert!(!is_valid_field_name("_"));
        assert!(!is_valid_field_name("_field"));
        assert!(!is_valid_field_name("__internal"));
        assert!(!is_valid_field_name("_123"));

        // Edge cases with special characters
        assert!(is_valid_field_name("field-name"));
        assert!(is_valid_field_name("field.name"));
        assert!(is_valid_field_name("field name")); // Spaces are allowed in names
    }

    #[test]
    fn test_filter_system_fields_empty() {
        let names: Vec<String> = vec![];
        let filtered = filter_system_fields(&names);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_system_fields_all_system() {
        let names = vec![
            "_internal".to_string(),
            "_meta".to_string(),
            "_system".to_string(),
        ];
        let filtered = filter_system_fields(&names);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_system_fields_all_valid() {
        let names = vec![
            "field1".to_string(),
            "field2".to_string(),
            "field3".to_string(),
        ];
        let filtered = filter_system_fields(&names);
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered, names);
    }

    #[test]
    fn test_filter_system_fields_mixed() {
        let names = vec![
            "field1".to_string(),
            "_internal".to_string(),
            "field2".to_string(),
            "_meta".to_string(),
            "field3".to_string(),
            "_system".to_string(),
        ];
        let filtered = filter_system_fields(&names);
        assert_eq!(filtered.len(), 3);
        assert!(filtered.contains(&"field1".to_string()));
        assert!(filtered.contains(&"field2".to_string()));
        assert!(filtered.contains(&"field3".to_string()));
        assert!(!filtered.contains(&"_internal".to_string()));
        assert!(!filtered.contains(&"_meta".to_string()));
        assert!(!filtered.contains(&"_system".to_string()));
    }

    #[test]
    fn test_type_checking_empty_values() {
        // Empty vectors
        assert!(is_vector(&Value::VectorInt(vec![])));
        assert!(is_vector(&Value::VectorFloat(vec![])));
        assert!(is_vector(&Value::VectorBool(vec![])));
        assert!(is_vector(&Value::VectorString(vec![])));

        // Empty matrix
        assert!(is_matrix(&Value::Matrix(vec![])));

        // Scalars are never empty in the type sense
        assert!(is_scalar(&Value::ScalarInt(0)));
        assert!(is_scalar(&Value::ScalarString(String::new())));
    }

    #[test]
    fn test_type_checking_single_element_vectors() {
        assert!(is_vector(&Value::VectorInt(vec![1])));
        assert!(is_vector(&Value::VectorFloat(vec![1.0])));
        assert!(is_vector(&Value::VectorBool(vec![true])));
        assert!(is_vector(&Value::VectorString(vec!["a".to_string()])));
    }
}

