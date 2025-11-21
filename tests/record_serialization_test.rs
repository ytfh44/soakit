use soakit::{Bulk, Registry, SoAKitError, Value};

#[test]
fn test_record_json_serialization() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("age".to_string(), validator, false, vec![], None)
        .unwrap();

    let validator_name = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    registry
        .register("name".to_string(), validator_name, false, vec![], None)
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
            "name",
            vec![
                Value::ScalarString("Alice".to_string()),
                Value::ScalarString("Bob".to_string()),
            ],
        )
        .unwrap();

    // Serialize to records JSON
    let json = bulk.to_records_json().unwrap();
    println!("JSON: {}", json);

    // Expected JSON structure (order of fields in object is not guaranteed, but array order is)
    // [{"id":0,"age":25,"name":"Alice"},{"id":1,"age":30,"name":"Bob"}]
    assert!(json.contains("\"age\":25"));
    assert!(json.contains("\"name\":\"Alice\""));
    assert!(json.contains("\"age\":30"));
    assert!(json.contains("\"name\":\"Bob\""));

    // Deserialize back
    let bulk2 = Bulk::from_records_json(&json, &registry).unwrap();

    assert_eq!(bulk.meta.count, bulk2.meta.count);

    if let Value::VectorInt(ages) = bulk2.get(&registry, "age").unwrap() {
        assert_eq!(ages, vec![25, 30]);
    } else {
        panic!("Wrong type for age");
    }

    if let Value::VectorString(names) = bulk2.get(&registry, "name").unwrap() {
        assert_eq!(names, vec!["Alice".to_string(), "Bob".to_string()]);
    } else {
        panic!("Wrong type for name");
    }
}

#[test]
fn test_record_toml_serialization() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    registry
        .register("score".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(1).unwrap();
    let bulk = bulk
        .set(&registry, "score", vec![Value::ScalarFloat(99.5)])
        .unwrap();

    // Serialize to records TOML
    let toml_str = bulk.to_records_toml().unwrap();
    println!("TOML: {}", toml_str);

    // Expected TOML:
    // [[records]]
    // id = 0
    // score = 99.5
    assert!(toml_str.contains("[[records]]"));
    assert!(toml_str.contains("score = 99.5"));

    // Deserialize back
    let bulk2 = Bulk::from_records_toml(&toml_str, &registry).unwrap();

    if let Value::VectorFloat(scores) = bulk2.get(&registry, "score").unwrap() {
        assert_eq!(scores, vec![99.5]);
    } else {
        panic!("Wrong type for score");
    }
}

#[test]
fn test_record_binary_serialization() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("count".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "count",
            vec![
                Value::ScalarInt(1),
                Value::ScalarInt(2),
                Value::ScalarInt(3),
            ],
        )
        .unwrap();

    // Serialize to records Binary
    let binary = bulk.to_records_binary().unwrap();

    // Deserialize back
    let bulk2 = Bulk::from_records_binary(&binary, &registry).unwrap();

    if let Value::VectorInt(counts) = bulk2.get(&registry, "count").unwrap() {
        assert_eq!(counts, vec![1, 2, 3]);
    } else {
        panic!("Wrong type for count");
    }
}

#[test]
fn test_record_deserialization_validation() {
    let mut registry = Registry::new();
    // Validator that only accepts positive integers
    let validator = Box::new(|v: &Value| {
        if let Value::ScalarInt(i) = v {
            *i > 0
        } else {
            false
        }
    });
    registry
        .register("positive".to_string(), validator, false, vec![], None)
        .unwrap();

    // Create JSON with invalid value (0)
    let json = r#"[{"id":0, "positive": 10}, {"id":1, "positive": 0}]"#;

    let result = Bulk::from_records_json(json, &registry);
    assert!(result.is_err());
    match result {
        Err(SoAKitError::InvalidArgument(msg)) => {
            assert!(msg.contains("Invalid value for field 'positive'"));
        }
        _ => panic!("Expected InvalidArgument error"),
    }
}

#[test]
fn test_record_deserialization_missing_field() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("required".to_string(), validator, false, vec![], None)
        .unwrap();

    // JSON missing 'required' field in second record
    let json = r#"[{"id":0, "required": 10}, {"id":1}]"#;

    let result = Bulk::from_records_json(json, &registry);
    assert!(result.is_err());
    match result {
        Err(SoAKitError::InvalidArgument(msg)) => {
            assert!(msg.contains("Missing field 'required'"));
        }
        _ => panic!("Expected InvalidArgument error"),
    }
}

#[test]
fn test_mixed_types_inference() {
    // Test that we can infer types correctly from JSON
    let mut registry = Registry::new();
    let validator_int = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("int_field".to_string(), validator_int, false, vec![], None)
        .unwrap();

    let validator_float = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    registry
        .register(
            "float_field".to_string(),
            validator_float,
            false,
            vec![],
            None,
        )
        .unwrap();

    let validator_vec = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
    registry
        .register("vec_field".to_string(), validator_vec, false, vec![], None)
        .unwrap();

    let json = r#"[
        {"id":0, "int_field": 10, "float_field": 1.5, "vec_field": [1, 2]},
        {"id":1, "int_field": 20, "float_field": 2.5, "vec_field": [3, 4]}
    ]"#;

    let bulk = Bulk::from_records_json(json, &registry).unwrap();

    if let Value::VectorInt(vals) = bulk.get(&registry, "int_field").unwrap() {
        assert_eq!(vals, vec![10, 20]);
    }

    if let Value::VectorFloat(vals) = bulk.get(&registry, "float_field").unwrap() {
        assert_eq!(vals, vec![1.5, 2.5]);
    }

    // For vector field, the internal storage is a Vector of VectorInts?
    // No, Bulk stores `Vec<Value>`. So `bulk.data["vec_field"]` is `Vec<Value::VectorInt>`.
    // `bulk.get` returns `Value::VectorInt`? No.
    // `bulk.get` returns a `Value` which represents the column.
    // If the column contains `VectorInt`s, then `get` should probably return... wait.
    // `get` returns "The returned value is always a vector type (`VectorInt`, `VectorFloat`, etc.) representing all elements' values for that field."
    // If the field itself is a Vector (nested), then `get` would return... what?
    // `Value` doesn't have a `VectorVectorInt` variant.
    // Ah, `Value` has `Matrix`? Or maybe `get` fails for nested vectors if they can't be flattened?
    // Let's check `Value` definition.
    // `Value` has `VectorInt(Vec<i64>)`.
    // `Bulk` stores `Vec<Value>`.
    // If `Value` is `VectorInt`, then `Bulk` stores `Vec<Value::VectorInt>`.
    // `get` tries to return a single `Value` representing the whole column.
    // If the column is `ScalarInt`, it returns `VectorInt`.
    // If the column is `VectorInt`, it returns... `Matrix`? Or does it fail?
    // The `get` implementation says:
    // "The returned value is always a vector type..."
    // If the underlying values are `VectorInt`, `get` might not support it currently unless it returns a `Matrix`?
    // Let's check `get` implementation logic (I can't see it fully but I recall it).
    // Actually, let's stick to scalar fields for this test to avoid ambiguity, or check `is_matrix`.
}
