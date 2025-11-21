/// Error types for SoAKit operations.
///
/// This module defines the error types used throughout SoAKit. All operations
/// that can fail return a [`Result<T, SoAKitError>`](Result).
use std::fmt;

/// Main error type for SoAKit operations.
///
/// This enum represents all possible errors that can occur when using SoAKit.
/// Each variant includes context information to help diagnose the issue.
///
/// # Examples
///
/// ```rust
/// use soakit::SoAKitError;
///
/// // Invalid argument error
/// let err = SoAKitError::InvalidArgument("Field name cannot be empty".to_string());
///
/// // Index out of bounds error
/// let err = SoAKitError::IndexOutOfBounds { index: 10, max: 5 };
///
/// // Length mismatch error
/// let err = SoAKitError::LengthMismatch { expected: 10, actual: 5 };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum SoAKitError {
    /// Invalid function argument.
    ///
    /// This error occurs when a function receives an argument that violates
    /// the function's preconditions or constraints.
    ///
    /// # Examples
    ///
    /// - Empty field name
    /// - Field name starting with underscore (reserved for system fields)
    /// - Inconsistent arguments (e.g., derived field without dependencies)
    InvalidArgument(String),
    /// Field not found in registry.
    ///
    /// This error occurs when attempting to access or use a field that hasn't
    /// been registered in the registry.
    ///
    /// # Examples
    ///
    /// - Trying to get a field that doesn't exist
    /// - Setting a field that hasn't been registered
    /// - Referencing a dependency that doesn't exist
    FieldNotFound(String),
    /// Field validation failed.
    ///
    /// This error occurs when a value fails to pass the field's validator function.
    /// The validator is defined when the field is registered.
    ///
    /// # Examples
    ///
    /// - Setting an integer field with a float value
    /// - Setting a vector field with a scalar value
    /// - Value doesn't meet custom validation criteria
    ValidationFailed(String),
    /// Index out of bounds.
    ///
    /// This error occurs when attempting to access an element at an index
    /// that is beyond the valid range.
    ///
    /// # Fields
    ///
    /// * `index` - The index that was accessed
    /// * `max` - The maximum valid index (exclusive, so valid range is 0..max)
    ///
    /// # Examples
    ///
    /// - Accessing element 10 in a vector of length 5
    /// - Creating a Proxy with index >= bulk count
    IndexOutOfBounds {
        /// The index that was accessed
        index: usize,
        /// The maximum valid index (exclusive)
        max: usize,
    },
    /// Value length doesn't match bulk count.
    ///
    /// This error occurs when setting field values where the number of values
    /// doesn't match the number of elements in the bulk structure.
    ///
    /// # Fields
    ///
    /// * `expected` - The expected length (bulk count)
    /// * `actual` - The actual length of the provided values
    ///
    /// # Examples
    ///
    /// - Setting 5 values in a bulk with 10 elements
    /// - Setting 10 values in a bulk with 5 elements
    LengthMismatch {
        /// The expected length (bulk count)
        expected: usize,
        /// The actual length of provided values
        actual: usize,
    },
    /// Derived field missing dependencies.
    ///
    /// This error occurs when attempting to register a derived field without
    /// specifying its dependencies, or when a derived field's dependencies
    /// cannot be resolved.
    ///
    /// # Examples
    ///
    /// - Registering a derived field with empty dependencies
    /// - Derived field depends on a non-existent field
    DerivedFieldNoDeps(String),
    /// Field already registered.
    ///
    /// This error occurs when attempting to register a field with a name
    /// that is already in use in the registry.
    ///
    /// # Examples
    ///
    /// - Registering "age" twice
    /// - Attempting to overwrite an existing field
    FieldAlreadyExists(String),
}

impl fmt::Display for SoAKitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SoAKitError::InvalidArgument(msg) => {
                write!(f, "Invalid argument: {}", msg)
            }
            SoAKitError::FieldNotFound(field) => {
                write!(f, "Field not found: {}", field)
            }
            SoAKitError::ValidationFailed(msg) => {
                write!(f, "Validation failed: {}", msg)
            }
            SoAKitError::IndexOutOfBounds { index, max } => {
                write!(f, "Index {} out of bounds (max: {})", index, max)
            }
            SoAKitError::LengthMismatch { expected, actual } => {
                write!(
                    f,
                    "Length mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            SoAKitError::DerivedFieldNoDeps(field) => {
                write!(f, "Derived field '{}' has no dependencies", field)
            }
            SoAKitError::FieldAlreadyExists(field) => {
                write!(f, "Field '{}' already exists", field)
            }
        }
    }
}

impl std::error::Error for SoAKitError {}

/// Result type alias for SoAKit operations
pub type Result<T> = std::result::Result<T, SoAKitError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_invalid_argument_display() {
        let err = SoAKitError::InvalidArgument("test message".to_string());
        let display_str = format!("{}", err);
        assert_eq!(display_str, "Invalid argument: test message");
    }

    #[test]
    fn test_field_not_found_display() {
        let err = SoAKitError::FieldNotFound("age".to_string());
        let display_str = format!("{}", err);
        assert_eq!(display_str, "Field not found: age");
    }

    #[test]
    fn test_validation_failed_display() {
        let err = SoAKitError::ValidationFailed("value too large".to_string());
        let display_str = format!("{}", err);
        assert_eq!(display_str, "Validation failed: value too large");
    }

    #[test]
    fn test_index_out_of_bounds_display() {
        let err = SoAKitError::IndexOutOfBounds { index: 10, max: 5 };
        let display_str = format!("{}", err);
        assert_eq!(display_str, "Index 10 out of bounds (max: 5)");
    }

    #[test]
    fn test_length_mismatch_display() {
        let err = SoAKitError::LengthMismatch {
            expected: 10,
            actual: 5,
        };
        let display_str = format!("{}", err);
        assert_eq!(display_str, "Length mismatch: expected 10, got 5");
    }

    #[test]
    fn test_derived_field_no_deps_display() {
        let err = SoAKitError::DerivedFieldNoDeps("sum".to_string());
        let display_str = format!("{}", err);
        assert_eq!(display_str, "Derived field 'sum' has no dependencies");
    }

    #[test]
    fn test_field_already_exists_display() {
        let err = SoAKitError::FieldAlreadyExists("age".to_string());
        let display_str = format!("{}", err);
        assert_eq!(display_str, "Field 'age' already exists");
    }

    #[test]
    fn test_error_equality() {
        let err1 = SoAKitError::InvalidArgument("test".to_string());
        let err2 = SoAKitError::InvalidArgument("test".to_string());
        let err3 = SoAKitError::InvalidArgument("different".to_string());

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_error_clone() {
        let err = SoAKitError::FieldNotFound("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_debug() {
        let err = SoAKitError::IndexOutOfBounds { index: 5, max: 3 };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("IndexOutOfBounds"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("3"));
    }

    #[test]
    fn test_all_error_variants() {
        // Test all variants can be created and formatted
        let errors = vec![
            SoAKitError::InvalidArgument("msg".to_string()),
            SoAKitError::FieldNotFound("field".to_string()),
            SoAKitError::ValidationFailed("msg".to_string()),
            SoAKitError::IndexOutOfBounds { index: 0, max: 0 },
            SoAKitError::LengthMismatch {
                expected: 0,
                actual: 0,
            },
            SoAKitError::DerivedFieldNoDeps("field".to_string()),
            SoAKitError::FieldAlreadyExists("field".to_string()),
        ];

        for err in errors {
            // Test Display
            let _display = format!("{}", err);
            // Test Debug
            let _debug = format!("{:?}", err);
            // Test Error trait
            let _source = err.source();
        }
    }

    #[test]
    fn test_error_with_empty_strings() {
        let err1 = SoAKitError::InvalidArgument(String::new());
        assert_eq!(format!("{}", err1), "Invalid argument: ");

        let err2 = SoAKitError::FieldNotFound(String::new());
        assert_eq!(format!("{}", err2), "Field not found: ");
    }

    #[test]
    fn test_error_with_special_characters() {
        let err = SoAKitError::InvalidArgument("test\nmessage\twith\rspecial".to_string());
        let display_str = format!("{}", err);
        assert!(display_str.contains("test"));
        assert!(display_str.contains("message"));
    }

    #[test]
    fn test_index_out_of_bounds_edge_cases() {
        // Zero index, zero max
        let err1 = SoAKitError::IndexOutOfBounds { index: 0, max: 0 };
        assert_eq!(format!("{}", err1), "Index 0 out of bounds (max: 0)");

        // Large values
        let err2 = SoAKitError::IndexOutOfBounds {
            index: usize::MAX,
            max: 100,
        };
        let display_str = format!("{}", err2);
        assert!(display_str.contains("out of bounds"));
    }

    #[test]
    fn test_length_mismatch_edge_cases() {
        // Zero values
        let err1 = SoAKitError::LengthMismatch {
            expected: 0,
            actual: 0,
        };
        assert_eq!(format!("{}", err1), "Length mismatch: expected 0, got 0");

        // Large values
        let err2 = SoAKitError::LengthMismatch {
            expected: usize::MAX,
            actual: 0,
        };
        let display_str = format!("{}", err2);
        assert!(display_str.contains("Length mismatch"));
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = SoAKitError::InvalidArgument("test".to_string());
        // Verify it implements Error trait
        let _: &dyn std::error::Error = &err;
        // source() should return None for our errors
        assert!(err.source().is_none());
    }

    #[test]
    fn test_error_type_matching() {
        let err = SoAKitError::FieldNotFound("test".to_string());
        match err {
            SoAKitError::FieldNotFound(_) => {}
            _ => panic!("Wrong error type"),
        }

        let err2 = SoAKitError::IndexOutOfBounds { index: 5, max: 3 };
        match err2 {
            SoAKitError::IndexOutOfBounds { index, max } => {
                assert_eq!(index, 5);
                assert_eq!(max, 3);
            }
            _ => panic!("Wrong error type"),
        }
    }
}

