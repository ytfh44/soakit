/// Integration tests for SoAKit
use soakit::*;
mod common;

#[test]
fn test_basic_workflow() {
    // Create registry
    let mut registry = Registry::new();
    
    // Register fields
    let age_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("age".to_string(), age_validator, false, vec![], None)
        .unwrap();
    
    let name_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    registry
        .register("name".to_string(), name_validator, false, vec![], None)
        .unwrap();
    
    // Create bulk
    let bulk = Bulk::new(3).unwrap();
    
    // Set field values
    let ages = vec![
        Value::ScalarInt(25),
        Value::ScalarInt(30),
        Value::ScalarInt(35),
    ];
    let bulk = bulk.set(&registry, "age", ages).unwrap();
    
    let names = vec![
        Value::ScalarString("Alice".to_string()),
        Value::ScalarString("Bob".to_string()),
        Value::ScalarString("Charlie".to_string()),
    ];
    let bulk = bulk.set(&registry, "name", names).unwrap();
    
    // Get field values
    let age_values = bulk.get(&registry, "age").unwrap();
    if let Value::VectorInt(v) = age_values {
        assert_eq!(v, vec![25, 30, 35]);
    } else {
        panic!("Expected VectorInt");
    }
    
    // Create proxy for single element
    let proxy = bulk.at(1).unwrap();
    let age = proxy.get_field(&registry, "age").unwrap();
    assert_eq!(age, Value::ScalarInt(30));
}

#[test]
fn test_derived_fields() {
    let mut registry = Registry::new();
    
    // Register base fields
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("a".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("b".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    
    // Register derived field
    let derived_func = Box::new(|args: &[Value]| {
        if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
            let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
            Ok(Value::VectorInt(sum))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });
    registry
        .register(
            "sum".to_string(),
            validator,
            true,
            vec!["a".to_string(), "b".to_string()],
            Some(derived_func),
        )
        .unwrap();
    
    // Create bulk and set base fields
    let bulk = Bulk::new(3).unwrap();
    let a_vals = vec![
        Value::ScalarInt(10),
        Value::ScalarInt(20),
        Value::ScalarInt(30),
    ];
    let bulk = bulk.set(&registry, "a", a_vals).unwrap();
    
    let b_vals = vec![
        Value::ScalarInt(5),
        Value::ScalarInt(15),
        Value::ScalarInt(25),
    ];
    let bulk = bulk.set(&registry, "b", b_vals).unwrap();
    
    // Get derived field
    let sum_values = bulk.get(&registry, "sum").unwrap();
    if let Value::VectorInt(v) = sum_values {
        assert_eq!(v, vec![15, 35, 55]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_apply_operation() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("age".to_string(), validator, false, vec![], None)
        .unwrap();
    
    let bulk = Bulk::new(5).unwrap();
    let ages = vec![
        Value::ScalarInt(10),
        Value::ScalarInt(20),
        Value::ScalarInt(30),
        Value::ScalarInt(40),
        Value::ScalarInt(50),
    ];
    let bulk = bulk.set(&registry, "age", ages).unwrap();
    
    // Apply function to masked subset
    let mask = vec![true, false, true, false, true];
    let new_bulk = bulk
        .apply(&mask, |subset| {
            let new_vals: Vec<Value> = subset
                .iter()
                .map(|v| {
                    if let Value::ScalarInt(i) = v {
                        Value::ScalarInt(i + 1)
                    } else {
                        v.clone()
                    }
                })
                .collect();
            Ok(new_vals)
        })
        .unwrap();
    
    let updated_ages = new_bulk.get(&registry, "age").unwrap();
    if let Value::VectorInt(v) = updated_ages {
        assert_eq!(v, vec![11, 20, 31, 40, 51]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_partition_by() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    registry
        .register("category".to_string(), validator, false, vec![], None)
        .unwrap();
    
    let bulk = Bulk::new(6).unwrap();
    let categories = vec![
        Value::ScalarString("A".to_string()),
        Value::ScalarString("B".to_string()),
        Value::ScalarString("A".to_string()),
        Value::ScalarString("C".to_string()),
        Value::ScalarString("B".to_string()),
        Value::ScalarString("A".to_string()),
    ];
    let bulk = bulk.set(&registry, "category", categories).unwrap();
    
    // Partition by category
    let views = bulk.partition_by(&registry, "category").unwrap();
    assert_eq!(views.len(), 3); // Three unique categories: A, B, C
    
    // Find view for category "A"
    let view_a = views
        .iter()
        .find(|v| {
            if let Value::ScalarString(s) = v.key() {
                s == "A"
            } else {
                false
            }
        })
        .unwrap();
    assert_eq!(view_a.count(), 3); // Three elements with category "A"
}

#[test]
fn test_error_handling() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("age".to_string(), validator, false, vec![], None)
        .unwrap();
    
    let bulk = Bulk::new(3).unwrap();
    
    // Test length mismatch
    let wrong_length = vec![Value::ScalarInt(10), Value::ScalarInt(20)];
    let result = bulk.set(&registry, "age", wrong_length);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::LengthMismatch { .. }
    ));
    
    // Test field not found
    let result = bulk.get(&registry, "nonexistent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
    
    // Test index out of bounds
    let result = bulk.at(10);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::IndexOutOfBounds { .. }
    ));
}

#[test]
fn test_version_tracking() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("age".to_string(), validator, false, vec![], None)
        .unwrap();
    
    let bulk = Bulk::new(3).unwrap();
    let ages1 = vec![
        Value::ScalarInt(10),
        Value::ScalarInt(20),
        Value::ScalarInt(30),
    ];
    let bulk = bulk.set(&registry, "age", ages1).unwrap();
    assert_eq!(bulk.meta.versions.get("age"), Some(&1));
    
    let ages2 = vec![
        Value::ScalarInt(11),
        Value::ScalarInt(21),
        Value::ScalarInt(31),
    ];
    let bulk = bulk.set(&registry, "age", ages2).unwrap();
    assert_eq!(bulk.meta.versions.get("age"), Some(&2));
}

#[test]
fn test_complex_workflow_with_multiple_fields() {
    use common::fixtures::simple_bulk;
    use common::assertions::{assert_vector_int, assert_vector_string, assert_scalar_int};

    let (bulk, registry) = simple_bulk();

    // Test getting all fields
    assert_vector_int(&bulk.get(&registry, "age").unwrap(), &[25, 30, 35]);
    assert_vector_string(
        &bulk.get(&registry, "name").unwrap(),
        &["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()],
    );

    // Test proxy access
    let proxy = bulk.at(0).unwrap();
    assert_scalar_int(&proxy.get_field(&registry, "age").unwrap(), 25);
}

#[test]
fn test_workflow_with_views() {
    use common::fixtures::simple_bulk;

    let (bulk, registry) = simple_bulk();

    // Partition by age (though all ages are unique, so each gets its own view)
    let views = bulk.partition_by(&registry, "age").unwrap();
    assert_eq!(views.len(), 3);

    // Each view should have count 1
    for view in &views {
        assert_eq!(view.count(), 1);
    }
}

#[test]
fn test_multi_step_workflow() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    // Step 1: Create bulk
    let bulk = Bulk::new(5).unwrap();

    // Step 2: Set initial values
    let bulk = bulk
        .set(
            &registry,
            "value",
            vec![
                Value::ScalarInt(1),
                Value::ScalarInt(2),
                Value::ScalarInt(3),
                Value::ScalarInt(4),
                Value::ScalarInt(5),
            ],
        )
        .unwrap();

    // Step 3: Apply transformation
    let mask = vec![true, true, false, false, true];
    let bulk = bulk
        .apply(&mask, |subset| {
            Ok(subset
                .iter()
                .map(|v| {
                    if let Value::ScalarInt(i) = v {
                        Value::ScalarInt(i * 10)
                    } else {
                        v.clone()
                    }
                })
                .collect())
        })
        .unwrap();

    // Step 4: Verify results
    if let Value::VectorInt(v) = bulk.get(&registry, "value").unwrap() {
        assert_eq!(v, vec![10, 20, 3, 4, 50]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_workflow_with_all_value_types() {
    let mut registry = Registry::new();

    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let float_validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    let bool_validator = Box::new(|v: &Value| matches!(v, Value::ScalarBool(_)));
    let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));

    registry
        .register("age".to_string(), int_validator, false, vec![], None)
        .unwrap();
    registry
        .register("height".to_string(), float_validator, false, vec![], None)
        .unwrap();
    registry
        .register("active".to_string(), bool_validator, false, vec![], None)
        .unwrap();
    registry
        .register("name".to_string(), str_validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(2).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "age",
            vec![Value::ScalarInt(25), Value::ScalarInt(30)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "height",
            vec![Value::ScalarFloat(1.75), Value::ScalarFloat(1.80)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "active",
            vec![Value::ScalarBool(true), Value::ScalarBool(false)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "name",
            vec![
                Value::ScalarString("Alice".to_string()),
                Value::ScalarString("Bob".to_string()),
            ],
        )
        .unwrap();

    // Access via proxy
    let proxy0 = bulk.at(0).unwrap();
    assert_eq!(proxy0.get_field(&registry, "age").unwrap(), Value::ScalarInt(25));
    assert_eq!(
        proxy0.get_field(&registry, "height").unwrap(),
        Value::ScalarFloat(1.75)
    );
    assert_eq!(
        proxy0.get_field(&registry, "active").unwrap(),
        Value::ScalarBool(true)
    );
    assert_eq!(
        proxy0.get_field(&registry, "name").unwrap(),
        Value::ScalarString("Alice".to_string())
    );
}

#[test]
fn test_workflow_with_partition_and_view_access() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    registry
        .register("category".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(6).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "category",
            vec![
                Value::ScalarString("A".to_string()),
                Value::ScalarString("B".to_string()),
                Value::ScalarString("A".to_string()),
                Value::ScalarString("C".to_string()),
                Value::ScalarString("B".to_string()),
                Value::ScalarString("A".to_string()),
            ],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "value",
            vec![
                Value::ScalarString("v1".to_string()),
                Value::ScalarString("v2".to_string()),
                Value::ScalarString("v3".to_string()),
                Value::ScalarString("v4".to_string()),
                Value::ScalarString("v5".to_string()),
                Value::ScalarString("v6".to_string()),
            ],
        )
        .unwrap();

    let views = bulk.partition_by(&registry, "category").unwrap();
    assert_eq!(views.len(), 3);

    // Find view for category "A" and check its values
    let view_a = views
        .iter()
        .find(|v| {
            if let Value::ScalarString(s) = v.key() {
                s == "A"
            } else {
                false
            }
        })
        .unwrap();

    if let Value::VectorString(v) = view_a.get_field(&registry, "value").unwrap() {
        assert_eq!(v.len(), 3);
        assert_eq!(v, vec!["v1".to_string(), "v3".to_string(), "v6".to_string()]);
    } else {
        panic!("Expected VectorString");
    }
}

