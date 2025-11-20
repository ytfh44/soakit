//! Common test utilities and helpers for SoAKit tests
#![allow(dead_code)] // Test helpers may not all be used in every test file
use soakit::*;

/// Builder for creating test registries with common field types
pub struct RegistryBuilder {
    registry: Registry,
}

impl RegistryBuilder {
    /// Create a new registry builder
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
        }
    }

    /// Add an integer field
    pub fn with_int_field(mut self, name: &str) -> Self {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        self.registry
            .register(name.to_string(), validator, false, vec![], None)
            .unwrap();
        self
    }

    /// Add a float field
    pub fn with_float_field(mut self, name: &str) -> Self {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
        self.registry
            .register(name.to_string(), validator, false, vec![], None)
            .unwrap();
        self
    }

    /// Add a boolean field
    pub fn with_bool_field(mut self, name: &str) -> Self {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarBool(_)));
        self.registry
            .register(name.to_string(), validator, false, vec![], None)
            .unwrap();
        self
    }

    /// Add a string field
    pub fn with_string_field(mut self, name: &str) -> Self {
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
        self.registry
            .register(name.to_string(), validator, false, vec![], None)
            .unwrap();
        self
    }

    /// Add a derived field
    pub fn with_derived_field(
        mut self,
        name: &str,
        dependencies: Vec<String>,
        func: Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>,
    ) -> Self {
        let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
        self.registry
            .register(name.to_string(), validator, true, dependencies, Some(func))
            .unwrap();
        self
    }

    /// Build the registry
    pub fn build(self) -> Registry {
        self.registry
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating test bulk structures
pub struct BulkBuilder {
    bulk: Bulk,
    registry: Registry,
}

impl BulkBuilder {
    /// Create a new bulk builder
    pub fn new(count: usize, registry: Registry) -> Self {
        Self {
            bulk: Bulk::new(count).unwrap(),
            registry,
        }
    }

    /// Set an integer field
    pub fn with_int_field(mut self, name: &str, values: Vec<i64>) -> Self {
        let values: Vec<Value> = values.into_iter().map(Value::ScalarInt).collect();
        self.bulk = self.bulk.set(&self.registry, name, values).unwrap();
        self
    }

    /// Set a float field
    pub fn with_float_field(mut self, name: &str, values: Vec<f64>) -> Self {
        let values: Vec<Value> = values.into_iter().map(Value::ScalarFloat).collect();
        self.bulk = self.bulk.set(&self.registry, name, values).unwrap();
        self
    }

    /// Set a boolean field
    pub fn with_bool_field(mut self, name: &str, values: Vec<bool>) -> Self {
        let values: Vec<Value> = values.into_iter().map(Value::ScalarBool).collect();
        self.bulk = self.bulk.set(&self.registry, name, values).unwrap();
        self
    }

    /// Set a string field
    pub fn with_string_field(mut self, name: &str, values: Vec<String>) -> Self {
        let values: Vec<Value> = values.into_iter().map(Value::ScalarString).collect();
        self.bulk = self.bulk.set(&self.registry, name, values).unwrap();
        self
    }

    /// Build the bulk
    pub fn build(self) -> (Bulk, Registry) {
        (self.bulk, self.registry)
    }
}

/// Assertion helpers for testing
pub mod assertions {
    use super::*;

    /// Assert that a value is a VectorInt with specific contents
    pub fn assert_vector_int(value: &Value, expected: &[i64]) {
        if let Value::VectorInt(v) = value {
            assert_eq!(v, expected);
        } else {
            panic!("Expected VectorInt, got {:?}", value);
        }
    }

    /// Assert that a value is a VectorFloat with specific contents
    pub fn assert_vector_float(value: &Value, expected: &[f64]) {
        if let Value::VectorFloat(v) = value {
            assert_eq!(v, expected);
        } else {
            panic!("Expected VectorFloat, got {:?}", value);
        }
    }

    /// Assert that a value is a VectorBool with specific contents
    pub fn assert_vector_bool(value: &Value, expected: &[bool]) {
        if let Value::VectorBool(v) = value {
            assert_eq!(v, expected);
        } else {
            panic!("Expected VectorBool, got {:?}", value);
        }
    }

    /// Assert that a value is a VectorString with specific contents
    pub fn assert_vector_string(value: &Value, expected: &[String]) {
        if let Value::VectorString(v) = value {
            assert_eq!(v, expected);
        } else {
            panic!("Expected VectorString, got {:?}", value);
        }
    }

    /// Assert that a value is a ScalarInt
    pub fn assert_scalar_int(value: &Value, expected: i64) {
        if let Value::ScalarInt(v) = value {
            assert_eq!(*v, expected);
        } else {
            panic!("Expected ScalarInt({}), got {:?}", expected, value);
        }
    }

    /// Assert that a value is a ScalarFloat (with epsilon comparison)
    pub fn assert_scalar_float(value: &Value, expected: f64, epsilon: f64) {
        if let Value::ScalarFloat(v) = value {
            assert!((v - expected).abs() < epsilon, "Expected {}, got {}", expected, v);
        } else {
            panic!("Expected ScalarFloat({}), got {:?}", expected, value);
        }
    }
}

/// Test fixtures for common scenarios
pub mod fixtures {
    use super::*;

    /// Create a simple registry with age and name fields
    pub fn simple_registry() -> Registry {
        RegistryBuilder::new()
            .with_int_field("age")
            .with_string_field("name")
            .build()
    }

    /// Create a simple bulk with age and name data
    pub fn simple_bulk() -> (Bulk, Registry) {
        let registry = simple_registry();
        BulkBuilder::new(3, registry)
            .with_int_field("age", vec![25, 30, 35])
            .with_string_field("name", vec!["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()])
            .build()
    }

    /// Create a registry with derived field
    pub fn registry_with_derived() -> Registry {
        let sum_func = Box::new(|args: &[Value]| {
            if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
                let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
                Ok(Value::VectorInt(sum))
            } else {
                Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
            }
        });

        RegistryBuilder::new()
            .with_int_field("a")
            .with_int_field("b")
            .with_derived_field("sum", vec!["a".to_string(), "b".to_string()], sum_func)
            .build()
    }
}

