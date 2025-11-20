/// Comprehensive error handling tests for SoAKit
use soakit::*;
mod common;

#[test]
fn test_bulk_creation_errors() {
    // Zero count should fail
    let result = Bulk::new(0);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::InvalidArgument(_)
    ));
}

#[test]
fn test_field_registration_errors() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

    // Duplicate registration
    registry
        .register("field".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    let result = registry.register("field".to_string(), validator, false, vec![], None);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::FieldAlreadyExists(_)
    ));

    // Invalid field name
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let result = registry.register("_invalid".to_string(), validator, false, vec![], None);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::InvalidArgument(_)
    ));

    // Empty field name
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let result = registry.register(String::new(), validator, false, vec![], None);
    assert!(result.is_err());
}

#[test]
fn test_derived_field_errors() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));

    // Derived field without dependencies
    let result = registry.register(
        "derived".to_string(),
        validator.clone(),
        true,
        vec![],
        None,
    );
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::DerivedFieldNoDeps(_)
    ));

    // Derived field without function
    let result = registry.register(
        "derived".to_string(),
        validator.clone(),
        true,
        vec!["a".to_string()],
        None,
    );
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::InvalidArgument(_)
    ));

    // Regular field with dependencies (should fail)
    let result = registry.register(
        "regular".to_string(),
        validator.clone(),
        false,
        vec!["a".to_string()],
        None,
    );
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::InvalidArgument(_)
    ));
}

#[test]
fn test_bulk_set_errors() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("age".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();

    // Field not found
    let values = vec![
        Value::ScalarInt(10),
        Value::ScalarInt(20),
        Value::ScalarInt(30),
    ];
    let result = bulk.set(&registry, "nonexistent", values);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));

    // Length mismatch
    let values = vec![Value::ScalarInt(10), Value::ScalarInt(20)];
    let result = bulk.set(&registry, "age", values);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::LengthMismatch { .. }
    ));

    // Validation failure
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let mut registry2 = Registry::new();
    registry2
        .register("age".to_string(), validator, false, vec![], None)
        .unwrap();
    let values = vec![
        Value::ScalarFloat(10.0),
        Value::ScalarFloat(20.0),
        Value::ScalarFloat(30.0),
    ];
    let result = bulk.set(&registry2, "age", values);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::ValidationFailed(_)
    ));
}

#[test]
fn test_bulk_get_errors() {
    let registry = Registry::new();
    let bulk = Bulk::new(3).unwrap();

    // Field not found
    let result = bulk.get(&registry, "nonexistent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
}

#[test]
fn test_proxy_errors() {
    let bulk = Bulk::new(5).unwrap();

    // Index out of bounds
    let result = bulk.at(10);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::IndexOutOfBounds { .. }
    ));

    // Index at boundary (should fail)
    let result = bulk.at(5);
    assert!(result.is_err());

    // Valid index
    let proxy = bulk.at(4).unwrap();
    assert_eq!(proxy.index(), 4);
}

#[test]
fn test_proxy_get_field_errors() {
    let registry = Registry::new();
    let bulk = Bulk::new(3).unwrap();
    let proxy = bulk.at(0).unwrap();

    // Field not found
    let result = proxy.get_field(&registry, "nonexistent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
}

#[test]
fn test_view_creation_errors() {
    let bulk = Bulk::new(5).unwrap();
    let mask = vec![true, false]; // Wrong length

    let result = View::new(Value::ScalarInt(0), mask, std::rc::Rc::new(bulk));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::LengthMismatch { .. }
    ));
}

#[test]
fn test_view_get_field_errors() {
    let registry = Registry::new();
    let bulk = Bulk::new(3).unwrap();
    let mask = vec![true, false, true];
    let view = View::new(Value::ScalarInt(0), mask, std::rc::Rc::new(bulk)).unwrap();

    // Field not found
    let result = view.get_field(&registry, "nonexistent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
}

#[test]
fn test_apply_operation_errors() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("age".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "age",
            vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
        )
        .unwrap();

    // Mask length mismatch
    let mask = vec![true, false];
    let result = bulk.apply(&mask, |subset| Ok(subset.to_vec()));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::LengthMismatch { .. }
    ));
}

#[test]
fn test_partition_errors() {
    let registry = Registry::new();
    let bulk = Bulk::new(3).unwrap();

    // Field not found
    let result = bulk.partition_by(&registry, "nonexistent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
}

#[test]
fn test_derived_field_missing_dependency() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));

    // Register derived field that depends on non-existent field
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

    let bulk = Bulk::new(2).unwrap();
    // Try to get derived field without setting dependencies
    let result = bulk.get(&registry, "sum");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
}

#[test]
fn test_value_get_element_errors() {
    // Get element from non-vector
    let scalar = Value::ScalarInt(42);
    let result = scalar.get_element(0);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::InvalidArgument(_)
    ));

    // Get element out of bounds
    let vector = Value::VectorInt(vec![1, 2, 3]);
    let result = vector.get_element(10);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SoAKitError::IndexOutOfBounds { .. }
    ));
}

#[test]
fn test_global_registry_errors() {
    // Test invalid field registration
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let result = register_field("_invalid".to_string(), validator, false, vec![], None);
    assert!(result.is_err());
}

