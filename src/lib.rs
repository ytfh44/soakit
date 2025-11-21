//! SoAKit - Structure-of-Arrays Kit
//!
//! A Rust implementation of a Structure-of-Arrays (SoA) data management system
//! with field metadata, derived fields with caching, versioning, and various access patterns.
//!
//! ## Overview
//!
//! SoAKit provides a high-performance data structure for managing structured data using
//! the Structure-of-Arrays (SoA) pattern. Instead of storing data as an array of structs (AoS),
//! SoAKit stores each field as a separate array, enabling better cache locality and
//! vectorized operations.
//!
//! ## Key Features
//!
//! - **Field Metadata System**: Register fields with validators and type information
//! - **Derived Fields**: Compute fields from other fields with automatic caching
//! - **Versioning**: Track changes to fields for cache invalidation
//! - **Multiple Access Patterns**: Bulk operations, single element access (Proxy), and partitioned views
//! - **Type Safety**: Strong typing with validation at runtime
//!
//! ## Quick Start
//!
//! ```rust
//! use soakit::{init, register_field, get_registry, Value};
//!
//! // Register a field
//! let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
//! register_field("age".to_string(), validator, false, vec![], None).unwrap();
//!
//! // Create a bulk structure with 3 elements
//! let bulk = init(3).unwrap();
//!
//! // Get the registry and set values
//! let registry = get_registry();
//! let reg = registry.lock().unwrap();
//! let values = vec![
//!     Value::ScalarInt(25),
//!     Value::ScalarInt(30),
//!     Value::ScalarInt(35),
//! ];
//! let bulk = bulk.set(&reg, "age", values).unwrap();
//!
//! // Retrieve values
//! if let Value::VectorInt(ages) = bulk.get(&reg, "age").unwrap() {
//!     println!("Ages: {:?}", ages);
//! }
//! ```
//!
//! ## Modules
//!
//! - [`bulk`]: Core Bulk data structure for SoA operations
//! - [`value`]: Value types (scalars, vectors, matrices)
//! - [`meta`]: Field metadata and registry
//! - [`view`]: Partitioned data views
//! - [`proxy`]: Single element access
//! - [`error`]: Error types
//! - [`util`]: Utility functions

pub mod bulk;
pub mod error;
pub mod meta;
pub mod proxy;
pub mod util;
pub mod value;
pub mod view;

// Re-export public API
pub use bulk::{Bulk, CacheEntry, Meta};
pub use error::{Result, SoAKitError};
pub use meta::{DerivedFunc, FieldMetadata, Registry};
pub use proxy::Proxy;
pub use util::{filter_system_fields, is_matrix, is_scalar, is_valid_field_name, is_vector};
pub use value::Value;
pub use view::View;

// Global registry instance using OnceLock for thread-safe singleton
use std::sync::OnceLock;

/// Global registry instance
static GLOBAL_REGISTRY: OnceLock<std::sync::Mutex<Registry>> = OnceLock::new();

/// Get or initialize the global registry.
///
/// The global registry is a thread-safe singleton that stores field metadata
/// for the entire application. All fields registered via [`register_field`] are
/// stored in this registry.
///
/// # Returns
///
/// A reference to the global registry wrapped in a `Mutex` for thread-safe access.
///
/// # Examples
///
/// ```rust
/// use soakit::{get_registry, Registry};
///
/// let registry = get_registry();
/// let reg = registry.lock().unwrap();
/// // Use the registry to check for fields, validate values, etc.
/// ```
pub fn get_registry() -> &'static std::sync::Mutex<Registry> {
    GLOBAL_REGISTRY.get_or_init(|| std::sync::Mutex::new(Registry::new()))
}

/// Register a field in the global registry.
///
/// Fields must be registered before they can be used in a [`Bulk`] structure.
/// Each field requires a validator function that checks if a value is valid for that field.
///
/// # Arguments
///
/// * `name` - The name of the field. Must not start with underscore (reserved for system fields)
///   and must not be empty.
/// * `validator` - A function that validates values for this field. Should return `true` if
///   the value is valid, `false` otherwise.
/// * `is_derived` - Whether this field is derived (computed from other fields).
/// * `dependencies` - For derived fields, the names of fields this field depends on.
///   Must be non-empty if `is_derived` is `true`.
/// * `derived_func` - For derived fields, the function that computes the field value
///   from its dependencies. Must be `Some` if `is_derived` is `true`.
///
/// # Returns
///
/// Returns `Ok(())` if the field was successfully registered, or an error if:
/// - The field name is invalid (starts with `_` or is empty)
/// - The field already exists
/// - For derived fields: dependencies are empty or `derived_func` is `None`
/// - For regular fields: dependencies are not empty or `derived_func` is `Some`
///
/// # Errors
///
/// - [`SoAKitError::InvalidArgument`] if the field name is invalid or arguments are inconsistent
/// - [`SoAKitError::FieldAlreadyExists`] if a field with the same name is already registered
/// - [`SoAKitError::DerivedFieldNoDeps`] if a derived field has no dependencies
///
/// # Examples
///
/// Registering a regular field:
///
/// ```rust
/// use soakit::{register_field, Value};
///
/// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
/// register_field("age".to_string(), validator, false, vec![], None).unwrap();
/// ```
///
/// Registering a derived field:
///
/// ```rust
/// use soakit::{register_field, Value, Result, SoAKitError};
///
/// let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
/// let derived_func = Box::new(|args: &[Value]| {
///     if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
///         let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
///         Ok(Value::VectorInt(sum))
///     } else {
///         Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
///     }
/// });
/// register_field(
///     "sum".to_string(),
///     validator,
///     true,
///     vec!["a".to_string(), "b".to_string()],
///     Some(derived_func),
/// ).unwrap();
/// ```
pub fn register_field(
    name: String,
    validator: Box<dyn Fn(&Value) -> bool + Send + Sync>,
    is_derived: bool,
    dependencies: Vec<String>,
    derived_func: Option<Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>>,
) -> Result<()> {
    let registry = get_registry();
    let mut reg = registry
        .lock()
        .map_err(|_| SoAKitError::InvalidArgument("Failed to lock global registry".to_string()))?;
    reg.register(name, validator, is_derived, dependencies, derived_func)
}

/// Initialize a new Bulk structure with the specified number of elements.
///
/// This is a convenience function that creates a new [`Bulk`] structure.
/// All elements in the bulk will initially have no field data; fields must be
/// set using [`Bulk::set`] after registration.
///
/// # Arguments
///
/// * `count` - The number of elements in the bulk. Must be greater than 0.
///
/// # Returns
///
/// Returns `Ok(Bulk)` if successful, or an error if `count` is 0.
///
/// # Errors
///
/// - [`SoAKitError::InvalidArgument`] if `count` is 0
///
/// # Examples
///
/// ```rust
/// use soakit::init;
///
/// // Create a bulk with 10 elements
/// let bulk = init(10).unwrap();
/// assert_eq!(bulk.count(), 10);
///
/// // Creating with 0 elements fails
/// assert!(init(0).is_err());
/// ```
pub fn init(count: usize) -> Result<Bulk> {
    Bulk::new(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        let bulk = init(5).unwrap();
        assert_eq!(bulk.count(), 5);
    }

    #[test]
    fn test_register_field() {
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        register_field("test".to_string(), validator, false, vec![], None).unwrap();
        let registry = get_registry();
        let reg = registry.lock().unwrap();
        assert!(reg.has_field("test"));
    }

    #[test]
    fn test_init_various_sizes() {
        let bulk1 = init(1).unwrap();
        assert_eq!(bulk1.count(), 1);

        let bulk10 = init(10).unwrap();
        assert_eq!(bulk10.count(), 10);

        let bulk100 = init(100).unwrap();
        assert_eq!(bulk100.count(), 100);
    }

    #[test]
    fn test_init_zero_fails() {
        let result = init(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_registry_singleton() {
        let registry1 = get_registry();
        let registry2 = get_registry();
        // They should be the same instance (same memory address)
        assert!(std::ptr::eq(registry1, registry2));
    }

    #[test]
    fn test_register_field_regular() {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        register_field("age".to_string(), validator, false, vec![], None).unwrap();
        let registry = get_registry();
        let reg = registry.lock().unwrap();
        assert!(reg.has_field("age"));
        assert!(!reg.is_empty());
    }

    #[test]
    fn test_register_field_derived() {
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        let derived_func = Box::new(|args: &[Value]| {
            if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
                let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
                Ok(Value::VectorInt(sum))
            } else {
                Err(SoAKitError::InvalidArgument(
                    "Invalid arguments".to_string(),
                ))
            }
        });
        register_field(
            "sum".to_string(),
            validator,
            true,
            vec!["a".to_string(), "b".to_string()],
            Some(derived_func),
        )
        .unwrap();
        let registry = get_registry();
        let reg = registry.lock().unwrap();
        assert!(reg.has_field("sum"));
    }

    #[test]
    fn test_register_field_duplicate_fails() {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        register_field(
            "duplicate".to_string(),
            validator.clone(),
            false,
            vec![],
            None,
        )
        .unwrap();
        let result = register_field("duplicate".to_string(), validator, false, vec![], None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::FieldAlreadyExists(_)
        ));
    }

    #[test]
    fn test_register_field_invalid_name() {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        let result = register_field("_invalid".to_string(), validator, false, vec![], None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::InvalidArgument(_)
        ));
    }

    #[test]
    fn test_register_field_empty_name() {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        let result = register_field(String::new(), validator, false, vec![], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_multiple_fields() {
        let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));

        register_field("field1".to_string(), int_validator, false, vec![], None).unwrap();
        register_field("field2".to_string(), str_validator, false, vec![], None).unwrap();

        let registry = get_registry();
        let reg = registry.lock().unwrap();
        assert!(reg.has_field("field1"));
        assert!(reg.has_field("field2"));
        assert!(reg.len() >= 2);
    }

    #[test]
    fn test_init_and_use_with_global_registry() {
        // Register a field in global registry
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        register_field("value".to_string(), validator, false, vec![], None).unwrap();

        // Create bulk
        let bulk = init(3).unwrap();

        // Get registry and use it
        let registry = get_registry();
        let reg = registry.lock().unwrap();
        assert!(reg.has_field("value"));

        // Set values
        let values = vec![
            Value::ScalarInt(10),
            Value::ScalarInt(20),
            Value::ScalarInt(30),
        ];
        let bulk = bulk.set(&reg, "value", values).unwrap();

        // Get values back
        if let Value::VectorInt(v) = bulk.get(&reg, "value").unwrap() {
            assert_eq!(v, vec![10, 20, 30]);
        } else {
            panic!("Expected VectorInt");
        }
    }
}
