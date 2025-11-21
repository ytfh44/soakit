/// Metadata registry for field definitions.
///
/// This module provides the [`Registry`] and [`FieldMetadata`] structures for
/// managing field definitions, validation, and derived field computation.
use crate::error::{Result, SoAKitError};
use crate::util::is_valid_field_name;
use crate::value::Value;
use std::collections::BTreeMap;

/// Metadata for a field in the registry.
///
/// Contains all information needed to validate and compute field values,
/// including the validator function, whether the field is derived, and
/// (for derived fields) the dependencies and computation function.
///
/// # Fields
///
/// * `validator` - Function that validates if a value is valid for this field
/// * `is_derived` - Whether this field is computed from other fields
/// * `dependencies` - For derived fields, the names of fields this depends on
/// * `derived_func` - For derived fields, the function that computes the value
pub struct FieldMetadata {
    /// Validator function that checks if a value is valid for this field
    pub validator: Box<dyn Fn(&Value) -> bool + Send + Sync>,
    /// Whether this field is derived (computed from other fields)
    pub is_derived: bool,
    /// Dependencies for derived fields (field names this field depends on)
    pub dependencies: Vec<String>,
    /// Function to compute derived field value from dependencies
    pub derived_func: Option<Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>>,
}

impl FieldMetadata {
    /// Create a new field metadata for a regular (non-derived) field.
    ///
    /// Regular fields store data directly and are not computed from other fields.
    ///
    /// # Arguments
    ///
    /// * `validator` - Function that validates values for this field
    ///
    /// # Returns
    ///
    /// A new `FieldMetadata` instance for a regular field.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::FieldMetadata;
    /// use soakit::Value;
    ///
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// let metadata = FieldMetadata::new(validator);
    /// assert!(!metadata.is_derived);
    /// ```
    pub fn new(validator: Box<dyn Fn(&Value) -> bool + Send + Sync>) -> Self {
        Self {
            validator,
            is_derived: false,
            dependencies: Vec::new(),
            derived_func: None,
        }
    }

    /// Create a new field metadata for a derived field.
    ///
    /// Derived fields are computed from other fields using the provided function.
    /// The dependencies must be non-empty.
    ///
    /// # Arguments
    ///
    /// * `validator` - Function that validates computed values for this field
    /// * `dependencies` - Names of fields this field depends on (must be non-empty)
    /// * `derived_func` - Function that computes the field value from dependencies
    ///
    /// # Returns
    ///
    /// Returns `Ok(FieldMetadata)` if successful, or an error if dependencies are empty.
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::DerivedFieldNoDeps`] if dependencies is empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::FieldMetadata;
    /// use soakit::{Value, Result, SoAKitError};
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
    /// let metadata = FieldMetadata::new_derived(
    ///     validator,
    ///     vec!["a".to_string(), "b".to_string()],
    ///     derived_func,
    /// ).unwrap();
    /// assert!(metadata.is_derived);
    /// ```
    pub fn new_derived(
        validator: Box<dyn Fn(&Value) -> bool + Send + Sync>,
        dependencies: Vec<String>,
        derived_func: Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>,
    ) -> Result<Self> {
        if dependencies.is_empty() {
            return Err(SoAKitError::DerivedFieldNoDeps(
                "Derived field must have dependencies".to_string(),
            ));
        }
        Ok(Self {
            validator,
            is_derived: true,
            dependencies,
            derived_func: Some(derived_func),
        })
    }
}

/// Registry for field metadata.
///
/// The registry stores metadata for all fields that can be used in [`Bulk`] structures.
/// It provides methods to register fields, validate values, and query field information.
///
/// Fields can be either regular (storing data directly) or derived (computed from
/// other fields). Derived fields automatically cache their computed values and
/// invalidate the cache when dependencies change.
pub struct Registry {
    fields: BTreeMap<String, FieldMetadata>,
}

impl Registry {
    /// Create a new empty registry.
    ///
    /// # Returns
    ///
    /// A new empty `Registry` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    ///
    /// let registry = Registry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    /// Register a new field.
    ///
    /// Registers a field with the given metadata. The field must have a valid name
    /// (not starting with `_` and not empty) and must not already exist.
    ///
    /// For derived fields, dependencies must be non-empty and a derived function
    /// must be provided. For regular fields, dependencies must be empty and no
    /// derived function should be provided.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the field (must be valid and unique)
    /// * `validator` - Function that validates values for this field
    /// * `is_derived` - Whether this is a derived field
    /// * `dependencies` - For derived fields, the names of fields this depends on
    /// * `derived_func` - For derived fields, the computation function
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful, or an error if:
    /// - The field name is invalid
    /// - The field already exists
    /// - Arguments are inconsistent (e.g., derived field without dependencies)
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::InvalidArgument`] if the name is invalid or arguments are inconsistent
    /// - [`SoAKitError::FieldAlreadyExists`] if the field already exists
    /// - [`SoAKitError::DerivedFieldNoDeps`] if a derived field has no dependencies
    ///
    /// # Examples
    ///
    /// Registering a regular field:
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    /// use soakit::Value;
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    /// ```
    ///
    /// Registering a derived field:
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    /// use soakit::{Value, Result, SoAKitError};
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
    /// let derived_func = Box::new(|args: &[Value]| {
    ///     if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
    ///         let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
    ///         Ok(Value::VectorInt(sum))
    ///     } else {
    ///         Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
    ///     }
    /// });
    /// registry.register(
    ///     "sum".to_string(),
    ///     validator,
    ///     true,
    ///     vec!["a".to_string(), "b".to_string()],
    ///     Some(derived_func),
    /// ).unwrap();
    /// ```
    pub fn register(
        &mut self,
        name: String,
        validator: Box<dyn Fn(&Value) -> bool + Send + Sync>,
        is_derived: bool,
        dependencies: Vec<String>,
        derived_func: Option<Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>>,
    ) -> Result<()> {
        if !is_valid_field_name(&name) {
            return Err(SoAKitError::InvalidArgument(format!(
                "Invalid field name: {}",
                name
            )));
        }

        if self.fields.contains_key(&name) {
            return Err(SoAKitError::FieldAlreadyExists(name));
        }

        if is_derived {
            if dependencies.is_empty() {
                return Err(SoAKitError::DerivedFieldNoDeps(name));
            }
            let derived_func = derived_func.ok_or_else(|| {
                SoAKitError::InvalidArgument(
                    "Derived field must have a derived function".to_string(),
                )
            })?;
            let metadata = FieldMetadata::new_derived(validator, dependencies, derived_func)?;
            let _ = self.fields.insert(name, metadata);
        } else {
            if !dependencies.is_empty() || derived_func.is_some() {
                return Err(SoAKitError::InvalidArgument(
                    "Non-derived field cannot have dependencies or derived function".to_string(),
                ));
            }
            let metadata = FieldMetadata::new(validator);
            let _ = self.fields.insert(name, metadata);
        }

        Ok(())
    }

    /// Validate a value against a field's validator.
    ///
    /// Checks if a value is valid for the specified field using the field's
    /// validator function.
    ///
    /// # Arguments
    ///
    /// * `field` - The name of the field to validate against
    /// * `value` - The value to validate
    ///
    /// # Returns
    ///
    /// Returns `true` if the field exists and the value passes validation,
    /// `false` if the field doesn't exist or validation fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    /// use soakit::Value;
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// assert!(registry.validate("age", &Value::ScalarInt(25)));
    /// assert!(!registry.validate("age", &Value::ScalarFloat(25.0)));
    /// assert!(!registry.validate("nonexistent", &Value::ScalarInt(25)));
    /// ```
    pub fn validate(&self, field: &str, value: &Value) -> bool {
        self.fields
            .get(field)
            .map(|meta| (meta.validator)(value))
            .unwrap_or(false)
    }

    /// Get metadata for a field.
    ///
    /// # Arguments
    ///
    /// * `field` - The name of the field
    ///
    /// # Returns
    ///
    /// Returns `Some(&FieldMetadata)` if the field exists, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    /// use soakit::Value;
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let metadata = registry.get_metadata("age");
    /// assert!(metadata.is_some());
    /// assert!(!metadata.unwrap().is_derived);
    /// ```
    pub fn get_metadata(&self, field: &str) -> Option<&FieldMetadata> {
        self.fields.get(field)
    }

    /// Check if a field exists in the registry.
    ///
    /// # Arguments
    ///
    /// * `field` - The name of the field to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the field is registered, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    /// use soakit::Value;
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// assert!(registry.has_field("age"));
    /// assert!(!registry.has_field("nonexistent"));
    /// ```
    pub fn has_field(&self, field: &str) -> bool {
        self.fields.contains_key(field)
    }

    /// List all registered field names (excluding system fields).
    ///
    /// # Returns
    ///
    /// A vector of all registered field names.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    /// use soakit::Value;
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator.clone(), false, vec![], None).unwrap();
    /// registry.register("height".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let fields = registry.list_fields();
    /// assert_eq!(fields.len(), 2);
    /// ```
    pub fn list_fields(&self) -> Vec<String> {
        self.fields.keys().cloned().collect()
    }

    /// Get the number of registered fields.
    ///
    /// # Returns
    ///
    /// The number of fields in the registry as a `usize`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    /// use soakit::Value;
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator.clone(), false, vec![], None).unwrap();
    /// registry.register("height".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// assert_eq!(registry.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check if the registry is empty.
    ///
    /// # Returns
    ///
    /// Returns `true` if no fields are registered, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::meta::Registry;
    ///
    /// let registry = Registry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_register_regular_field() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        reg.register("age".to_string(), validator, false, vec![], None)
            .unwrap();
        assert!(reg.has_field("age"));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_register_duplicate_field() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        reg.register("age".to_string(), validator.clone(), false, vec![], None)
            .unwrap();
        let result = reg.register("age".to_string(), validator, false, vec![], None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::FieldAlreadyExists(_)
        ));
    }

    #[test]
    fn test_register_derived_field() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        let derived_func = Box::new(|args: &[Value]| {
            if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
                let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
                Ok(Value::VectorInt(sum))
            } else {
                Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
            }
        });
        reg.register(
            "sum".to_string(),
            validator,
            true,
            vec!["a".to_string(), "b".to_string()],
            Some(derived_func),
        )
        .unwrap();
        assert!(reg.has_field("sum"));
    }

    #[test]
    fn test_register_derived_field_no_deps() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        let result = reg.register(
            "sum".to_string(),
            validator,
            true,
            vec![],
            None,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::DerivedFieldNoDeps(_)
        ));
    }

    #[test]
    fn test_validate() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        reg.register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let valid_value = Value::VectorInt(vec![1, 2, 3]);
        let invalid_value = Value::VectorFloat(vec![1.0, 2.0]);

        assert!(reg.validate("age", &valid_value));
        assert!(!reg.validate("age", &invalid_value));
        assert!(!reg.validate("nonexistent", &valid_value));
    }

    #[test]
    fn test_invalid_field_name() {
        let mut reg = Registry::new();
        let validator = Box::new(|_v: &Value| true);
        let result = reg.register("_internal".to_string(), validator, false, vec![], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_field_name_empty() {
        let mut reg = Registry::new();
        let validator = Box::new(|_v: &Value| true);
        let result = reg.register(String::new(), validator, false, vec![], None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::InvalidArgument(_)
        ));
    }

    #[test]
    fn test_register_multiple_fields() {
        let mut reg = Registry::new();
        let validator_int = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        let validator_str = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));

        reg.register("age".to_string(), validator_int, false, vec![], None)
            .unwrap();
        reg.register("name".to_string(), validator_str, false, vec![], None)
            .unwrap();

        assert_eq!(reg.len(), 2);
        assert!(reg.has_field("age"));
        assert!(reg.has_field("name"));
        assert!(!reg.is_empty());
    }

    #[test]
    fn test_register_regular_field_with_deps_should_fail() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        let result = reg.register(
            "age".to_string(),
            validator,
            false,
            vec!["other".to_string()],
            None,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::InvalidArgument(_)
        ));
    }

    #[test]
    fn test_register_regular_field_with_derived_func_should_fail() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        let derived_func = Box::new(|_args: &[Value]| Ok(Value::ScalarInt(0)));
        let result = reg.register(
            "age".to_string(),
            validator,
            false,
            vec![],
            Some(derived_func),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::InvalidArgument(_)
        ));
    }

    #[test]
    fn test_register_derived_field_without_func_should_fail() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        let result = reg.register(
            "sum".to_string(),
            validator,
            true,
            vec!["a".to_string(), "b".to_string()],
            None,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::InvalidArgument(_)
        ));
    }

    #[test]
    fn test_validate_with_different_validators() {
        let mut reg = Registry::new();

        // Integer validator
        let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        reg.register("int_field".to_string(), int_validator, false, vec![], None)
            .unwrap();

        // Float validator
        let float_validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
        reg.register("float_field".to_string(), float_validator, false, vec![], None)
            .unwrap();

        // String validator
        let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
        reg.register("str_field".to_string(), str_validator, false, vec![], None)
            .unwrap();

        // Bool validator
        let bool_validator = Box::new(|v: &Value| matches!(v, Value::ScalarBool(_)));
        reg.register("bool_field".to_string(), bool_validator, false, vec![], None)
            .unwrap();

        assert!(reg.validate("int_field", &Value::ScalarInt(42)));
        assert!(!reg.validate("int_field", &Value::ScalarFloat(3.14)));

        assert!(reg.validate("float_field", &Value::ScalarFloat(3.14)));
        assert!(!reg.validate("float_field", &Value::ScalarInt(42)));

        assert!(reg.validate("str_field", &Value::ScalarString("test".to_string())));
        assert!(!reg.validate("str_field", &Value::ScalarInt(42)));

        assert!(reg.validate("bool_field", &Value::ScalarBool(true)));
        assert!(!reg.validate("bool_field", &Value::ScalarInt(42)));
    }

    #[test]
    fn test_validate_with_complex_validator() {
        let mut reg = Registry::new();
        // Validator that checks if value is a vector with length > 0
        let validator = Box::new(|v: &Value| {
            if let Value::VectorInt(vec) = v {
                !vec.is_empty()
            } else {
                false
            }
        });
        reg.register("non_empty_vec".to_string(), validator, false, vec![], None)
            .unwrap();

        assert!(reg.validate("non_empty_vec", &Value::VectorInt(vec![1, 2, 3])));
        assert!(!reg.validate("non_empty_vec", &Value::VectorInt(vec![])));
        assert!(!reg.validate("non_empty_vec", &Value::ScalarInt(42)));
    }

    #[test]
    fn test_get_metadata() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        reg.register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let metadata = reg.get_metadata("age");
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert!(!meta.is_derived);
        assert!(meta.dependencies.is_empty());
        assert!(meta.derived_func.is_none());

        // Test validator works
        assert!((meta.validator)(&Value::ScalarInt(42)));
        assert!(!(meta.validator)(&Value::ScalarFloat(3.14)));
    }

    #[test]
    fn test_get_metadata_derived_field() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        let derived_func = Box::new(|args: &[Value]| {
            if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
                let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
                Ok(Value::VectorInt(sum))
            } else {
                Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
            }
        });
        reg.register(
            "sum".to_string(),
            validator,
            true,
            vec!["a".to_string(), "b".to_string()],
            Some(derived_func),
        )
        .unwrap();

        let metadata = reg.get_metadata("sum");
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert!(meta.is_derived);
        assert_eq!(meta.dependencies, vec!["a".to_string(), "b".to_string()]);
        assert!(meta.derived_func.is_some());
    }

    #[test]
    fn test_get_metadata_nonexistent() {
        let reg = Registry::new();
        let metadata = reg.get_metadata("nonexistent");
        assert!(metadata.is_none());
    }

    #[test]
    fn test_list_fields() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

        reg.register("field1".to_string(), validator.clone(), false, vec![], None)
            .unwrap();
        reg.register("field2".to_string(), validator.clone(), false, vec![], None)
            .unwrap();
        reg.register("field3".to_string(), validator, false, vec![], None)
            .unwrap();

        let fields = reg.list_fields();
        assert_eq!(fields.len(), 3);
        assert!(fields.contains(&"field1".to_string()));
        assert!(fields.contains(&"field2".to_string()));
        assert!(fields.contains(&"field3".to_string()));
    }

    #[test]
    fn test_list_fields_empty() {
        let reg = Registry::new();
        let fields = reg.list_fields();
        assert!(fields.is_empty());
    }

    #[test]
    fn test_registry_is_empty() {
        let mut reg = Registry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);

        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        reg.register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_registry_default() {
        let reg = Registry::default();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_field_metadata_new() {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        let metadata = FieldMetadata::new(validator);
        assert!(!metadata.is_derived);
        assert!(metadata.dependencies.is_empty());
        assert!(metadata.derived_func.is_none());
    }

    #[test]
    fn test_field_metadata_new_derived() {
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        let derived_func = Box::new(|args: &[Value]| {
            if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
                let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
                Ok(Value::VectorInt(sum))
            } else {
                Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
            }
        });
        let metadata = FieldMetadata::new_derived(
            validator,
            vec!["a".to_string(), "b".to_string()],
            derived_func,
        )
        .unwrap();

        assert!(metadata.is_derived);
        assert_eq!(metadata.dependencies, vec!["a".to_string(), "b".to_string()]);
        assert!(metadata.derived_func.is_some());
    }

    #[test]
    fn test_field_metadata_new_derived_no_deps() {
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        let derived_func = Box::new(|_args: &[Value]| Ok(Value::VectorInt(vec![])));
        let result = FieldMetadata::new_derived(validator, vec![], derived_func);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, SoAKitError::DerivedFieldNoDeps(_)));
        } else {
            panic!("Expected error");
        }
    }

    #[test]
    fn test_derived_field_with_multiple_dependencies() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        let derived_func = Box::new(|args: &[Value]| {
            if args.len() == 3 {
                if let (Value::VectorInt(a), Value::VectorInt(b), Value::VectorInt(c)) =
                    (&args[0], &args[1], &args[2])
                {
                    let sum: Vec<i64> = a
                        .iter()
                        .zip(b.iter())
                        .zip(c.iter())
                        .map(|((x, y), z)| x + y + z)
                        .collect();
                    Ok(Value::VectorInt(sum))
                } else {
                    Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
                }
            } else {
                Err(SoAKitError::InvalidArgument("Wrong number of args".to_string()))
            }
        });
        reg.register(
            "total".to_string(),
            validator,
            true,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            Some(derived_func),
        )
        .unwrap();

        let metadata = reg.get_metadata("total").unwrap();
        assert_eq!(metadata.dependencies.len(), 3);
    }

    #[test]
    fn test_register_field_with_special_characters_in_name() {
        let mut reg = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

        // Valid names with various characters
        reg.register("field_123".to_string(), validator.clone(), false, vec![], None)
            .unwrap();
        reg.register("field-name".to_string(), validator.clone(), false, vec![], None)
            .unwrap();
        reg.register("fieldName".to_string(), validator, false, vec![], None)
            .unwrap();

        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn test_validate_returns_false_for_nonexistent_field() {
        let reg = Registry::new();
        let value = Value::ScalarInt(42);
        assert!(!reg.validate("nonexistent", &value));
    }

    #[test]
    fn test_has_field() {
        let mut reg = Registry::new();
        assert!(!reg.has_field("age"));

        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        reg.register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        assert!(reg.has_field("age"));
        assert!(!reg.has_field("name"));
    }
}

