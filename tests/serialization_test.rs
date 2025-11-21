//! Tests for Bulk serialization and deserialization functionality.

use soakit::{Bulk, Registry, Value};

#[test]
fn test_json_round_trip() {
    // Create a registry and bulk
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("age".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let values = vec![
        Value::ScalarInt(25),
        Value::ScalarInt(30),
        Value::ScalarInt(35),
    ];
    let bulk = bulk.set(&registry, "age", values).unwrap();

    // Serialize to JSON
    let json = bulk.to_json().unwrap();
    assert!(!json.is_empty());

    // Deserialize from JSON
    let deserialized = Bulk::from_json(&json).unwrap();

    // Verify the meta fields match
    assert_eq!(deserialized.meta.count, bulk.meta.count);
    assert_eq!(deserialized.meta.id, bulk.meta.id);
    assert_eq!(deserialized.meta.versions, bulk.meta.versions);

    // Verify the data matches
    let original_ages = bulk.get(&registry, "age").unwrap();
    let deserialized_ages = deserialized.get(&registry, "age").unwrap();
    assert_eq!(deserialized_ages, original_ages);
}

#[test]
fn test_binary_round_trip() {
    // Create a registry and bulk
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    registry
        .register("height".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(2).unwrap();
    let values = vec![Value::ScalarFloat(1.75), Value::ScalarFloat(1.80)];
    let bulk = bulk.set(&registry, "height", values).unwrap();

    // Serialize to binary
    let binary = bulk.to_binary().unwrap();
    assert!(!binary.is_empty());

    // Deserialize from binary
    let deserialized = Bulk::from_binary(&binary).unwrap();

    // Verify the meta fields match
    assert_eq!(deserialized.meta.count, bulk.meta.count);
    assert_eq!(deserialized.meta.id, bulk.meta.id);
    assert_eq!(deserialized.meta.versions, bulk.meta.versions);

    // Verify the data matches
    let original_heights = bulk.get(&registry, "height").unwrap();
    let deserialized_heights = deserialized.get(&registry, "height").unwrap();
    assert_eq!(deserialized_heights, original_heights);
}

#[test]
fn test_cache_invalidation_after_deserialization() {
    // Create a registry with a derived field
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let derived_func = Box::new(
        |deps: &[Value]| -> Result<Value, soakit::error::SoAKitError> {
            if let Value::VectorInt(values) = &deps[0] {
                let doubled: Vec<i64> = values.iter().map(|x| x * 2).collect();
                Ok(Value::VectorInt(doubled))
            } else {
                Err(soakit::error::SoAKitError::InvalidArgument(
                    "Expected VectorInt".to_string(),
                ))
            }
        },
    );

    registry
        .register(
            "doubled".to_string(),
            Box::new(|v: &Value| matches!(v, Value::VectorInt(_))),
            true,
            vec!["value".to_string()],
            Some(derived_func),
        )
        .unwrap();

    // Create bulk and set values
    let bulk = Bulk::new(3).unwrap();
    let values = vec![
        Value::ScalarInt(10),
        Value::ScalarInt(20),
        Value::ScalarInt(30),
    ];
    let bulk = bulk.set(&registry, "value", values).unwrap();

    // Get derived field to populate cache
    let doubled_before = bulk.get(&registry, "doubled").unwrap();
    assert_eq!(doubled_before, Value::VectorInt(vec![20, 40, 60]));

    // Serialize and deserialize
    let json = bulk.to_json().unwrap();
    let deserialized = Bulk::from_json(&json).unwrap();

    // Cache should be empty after deserialization
    assert!(deserialized.cache.borrow().is_empty());

    // But derived field should still compute correctly
    let doubled_after = deserialized.get(&registry, "doubled").unwrap();
    assert_eq!(doubled_after, Value::VectorInt(vec![20, 40, 60]));

    // And now the cache should be populated
    assert!(!deserialized.cache.borrow().is_empty());
}

#[test]
fn test_multiple_fields_serialization() {
    // Create a registry with multiple fields
    let mut registry = Registry::new();
    registry
        .register(
            "age".to_string(),
            Box::new(|v: &Value| matches!(v, Value::ScalarInt(_))),
            false,
            vec![],
            None,
        )
        .unwrap();
    registry
        .register(
            "name".to_string(),
            Box::new(|v: &Value| matches!(v, Value::ScalarString(_))),
            false,
            vec![],
            None,
        )
        .unwrap();
    registry
        .register(
            "active".to_string(),
            Box::new(|v: &Value| matches!(v, Value::ScalarBool(_))),
            false,
            vec![],
            None,
        )
        .unwrap();

    // Create bulk and set multiple fields
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
            "name",
            vec![
                Value::ScalarString("Alice".to_string()),
                Value::ScalarString("Bob".to_string()),
            ],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "active",
            vec![Value::ScalarBool(true), Value::ScalarBool(false)],
        )
        .unwrap();

    // Test JSON round-trip
    let json = bulk.to_json().unwrap();
    let deserialized_json = Bulk::from_json(&json).unwrap();
    assert_eq!(
        deserialized_json.get(&registry, "age").unwrap(),
        bulk.get(&registry, "age").unwrap()
    );
    assert_eq!(
        deserialized_json.get(&registry, "name").unwrap(),
        bulk.get(&registry, "name").unwrap()
    );
    assert_eq!(
        deserialized_json.get(&registry, "active").unwrap(),
        bulk.get(&registry, "active").unwrap()
    );

    // Test binary round-trip
    let binary = bulk.to_binary().unwrap();
    let deserialized_binary = Bulk::from_binary(&binary).unwrap();
    assert_eq!(
        deserialized_binary.get(&registry, "age").unwrap(),
        bulk.get(&registry, "age").unwrap()
    );
    assert_eq!(
        deserialized_binary.get(&registry, "name").unwrap(),
        bulk.get(&registry, "name").unwrap()
    );
    assert_eq!(
        deserialized_binary.get(&registry, "active").unwrap(),
        bulk.get(&registry, "active").unwrap()
    );
}

#[test]
fn test_toml_round_trip() {
    // Create a registry and bulk
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("score".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let values = vec![
        Value::ScalarInt(100),
        Value::ScalarInt(85),
        Value::ScalarInt(92),
    ];
    let bulk = bulk.set(&registry, "score", values).unwrap();

    // Serialize to TOML
    let toml = bulk.to_toml().unwrap();
    assert!(!toml.is_empty());

    // Deserialize from TOML
    let deserialized = Bulk::from_toml(&toml).unwrap();

    // Verify the meta fields match
    assert_eq!(deserialized.meta.count, bulk.meta.count);
    assert_eq!(deserialized.meta.id, bulk.meta.id);
    assert_eq!(deserialized.meta.versions, bulk.meta.versions);

    // Verify the data matches
    let original_scores = bulk.get(&registry, "score").unwrap();
    let deserialized_scores = deserialized.get(&registry, "score").unwrap();
    assert_eq!(deserialized_scores, original_scores);
}
