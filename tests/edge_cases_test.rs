/// Comprehensive edge case tests for SoAKit
use soakit::*;
mod common;

#[test]
fn test_empty_bulk_creation() {
    // Should fail - bulk must have at least 1 element
    let result = Bulk::new(0);
    assert!(result.is_err());
}

#[test]
fn test_single_element_bulk() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(1).unwrap();
    let bulk = bulk.set(&registry, "value", vec![Value::ScalarInt(42)]).unwrap();

    if let Value::VectorInt(v) = bulk.get(&registry, "value").unwrap() {
        assert_eq!(v, vec![42]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_empty_vectors() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
    registry
        .register("empty".to_string(), validator, false, vec![], None)
        .unwrap();

    // Note: We can't actually set empty vectors in bulk since each element needs a value
    // But we can test empty vector values themselves
    let empty_vec = Value::VectorInt(vec![]);
    assert!(empty_vec.is_empty());
    assert_eq!(empty_vec.len(), 0);
}

#[test]
fn test_nan_handling() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "value",
            vec![
                Value::ScalarFloat(1.0),
                Value::ScalarFloat(f64::NAN),
                Value::ScalarFloat(3.0),
            ],
        )
        .unwrap();

    if let Value::VectorFloat(v) = bulk.get(&registry, "value").unwrap() {
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 1.0);
        assert!(v[1].is_nan());
        assert_eq!(v[2], 3.0);
    } else {
        panic!("Expected VectorFloat");
    }
}

#[test]
fn test_infinity_handling() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(2).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "value",
            vec![
                Value::ScalarFloat(f64::INFINITY),
                Value::ScalarFloat(f64::NEG_INFINITY),
            ],
        )
        .unwrap();

    if let Value::VectorFloat(v) = bulk.get(&registry, "value").unwrap() {
        assert!(v[0].is_infinite() && v[0].is_sign_positive());
        assert!(v[1].is_infinite() && v[1].is_sign_negative());
    } else {
        panic!("Expected VectorFloat");
    }
}

#[test]
fn test_empty_strings() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    registry
        .register("name".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "name",
            vec![
                Value::ScalarString(String::new()),
                Value::ScalarString("test".to_string()),
                Value::ScalarString(String::new()),
            ],
        )
        .unwrap();

    if let Value::VectorString(v) = bulk.get(&registry, "name").unwrap() {
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], String::new());
        assert_eq!(v[1], "test");
        assert_eq!(v[2], String::new());
    } else {
        panic!("Expected VectorString");
    }
}

#[test]
fn test_extreme_integer_values() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "value",
            vec![
                Value::ScalarInt(i64::MIN),
                Value::ScalarInt(0),
                Value::ScalarInt(i64::MAX),
            ],
        )
        .unwrap();

    if let Value::VectorInt(v) = bulk.get(&registry, "value").unwrap() {
        assert_eq!(v[0], i64::MIN);
        assert_eq!(v[1], 0);
        assert_eq!(v[2], i64::MAX);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_large_bulk() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let count = 10000;
    let bulk = Bulk::new(count).unwrap();
    let values: Vec<Value> = (0..count).map(|i| Value::ScalarInt(i as i64)).collect();
    let bulk = bulk.set(&registry, "value", values).unwrap();

    assert_eq!(bulk.count(), count);
    if let Value::VectorInt(v) = bulk.get(&registry, "value").unwrap() {
        assert_eq!(v.len(), count);
        assert_eq!(v[0], 0);
        assert_eq!(v[count - 1], (count - 1) as i64);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_all_false_mask() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "value",
            vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
        )
        .unwrap();

    let mask = vec![false, false, false];
    let new_bulk = bulk
        .apply(&mask, |subset| {
            assert_eq!(subset.len(), 0); // Should be empty
            Ok(vec![])
        })
        .unwrap();

    // Values should remain unchanged since mask is all false
    if let Value::VectorInt(v) = new_bulk.get(&registry, "value").unwrap() {
        assert_eq!(v, vec![10, 20, 30]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_all_true_mask() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "value",
            vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
        )
        .unwrap();

    let mask = vec![true, true, true];
    let new_bulk = bulk
        .apply(&mask, |subset| {
            Ok(subset
                .iter()
                .map(|v| {
                    if let Value::ScalarInt(i) = v {
                        Value::ScalarInt(i + 1)
                    } else {
                        v.clone()
                    }
                })
                .collect())
        })
        .unwrap();

    if let Value::VectorInt(v) = new_bulk.get(&registry, "value").unwrap() {
        assert_eq!(v, vec![11, 21, 31]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_empty_view() {
    let bulk = Bulk::new(3).unwrap();
    let mask = vec![false, false, false];
    let view = View::new(Value::ScalarInt(0), mask, std::rc::Rc::new(bulk)).unwrap();
    assert!(view.is_empty());
    assert_eq!(view.count(), 0);
}

#[test]
fn test_partition_with_duplicate_values() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("category".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(5).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "category",
            vec![
                Value::ScalarInt(1),
                Value::ScalarInt(2),
                Value::ScalarInt(1),
                Value::ScalarInt(1),
                Value::ScalarInt(2),
            ],
        )
        .unwrap();

    let views = bulk.partition_by(&registry, "category").unwrap();
    assert_eq!(views.len(), 2); // Two unique values: 1 and 2

    // Find view for category 1
    let view_1 = views
        .iter()
        .find(|v| {
            if let Value::ScalarInt(i) = v.key() {
                *i == 1
            } else {
                false
            }
        })
        .unwrap();
    assert_eq!(view_1.count(), 3); // Three elements with value 1
}

#[test]
fn test_partition_with_all_same_values() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("category".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(5).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "category",
            vec![
                Value::ScalarInt(1),
                Value::ScalarInt(1),
                Value::ScalarInt(1),
                Value::ScalarInt(1),
                Value::ScalarInt(1),
            ],
        )
        .unwrap();

    let views = bulk.partition_by(&registry, "category").unwrap();
    assert_eq!(views.len(), 1); // Only one unique value
    assert_eq!(views[0].count(), 5);
}

#[test]
fn test_zero_values() {
    let mut registry = Registry::new();
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let float_validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));

    registry
        .register("int_zero".to_string(), int_validator, false, vec![], None)
        .unwrap();
    registry
        .register("float_zero".to_string(), float_validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(2).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "int_zero",
            vec![Value::ScalarInt(0), Value::ScalarInt(0)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "float_zero",
            vec![Value::ScalarFloat(0.0), Value::ScalarFloat(-0.0)],
        )
        .unwrap();

    if let Value::VectorInt(v) = bulk.get(&registry, "int_zero").unwrap() {
        assert_eq!(v, vec![0, 0]);
    } else {
        panic!("Expected VectorInt");
    }

    if let Value::VectorFloat(v) = bulk.get(&registry, "float_zero").unwrap() {
        assert_eq!(v[0], 0.0);
        assert_eq!(v[1], -0.0);
    } else {
        panic!("Expected VectorFloat");
    }
}

#[test]
fn test_unicode_strings() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    registry
        .register("name".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "name",
            vec![
                Value::ScalarString("你好".to_string()),
                Value::ScalarString("世界".to_string()),
                Value::ScalarString("🌍".to_string()),
            ],
        )
        .unwrap();

    if let Value::VectorString(v) = bulk.get(&registry, "name").unwrap() {
        assert_eq!(v[0], "你好");
        assert_eq!(v[1], "世界");
        assert_eq!(v[2], "🌍");
    } else {
        panic!("Expected VectorString");
    }
}

#[test]
fn test_special_string_characters() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    registry
        .register("text".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "text",
            vec![
                Value::ScalarString("line1\nline2".to_string()),
                Value::ScalarString("tab\there".to_string()),
                Value::ScalarString("quote\"here".to_string()),
            ],
        )
        .unwrap();

    if let Value::VectorString(v) = bulk.get(&registry, "text").unwrap() {
        assert_eq!(v[0], "line1\nline2");
        assert_eq!(v[1], "tab\there");
        assert_eq!(v[2], "quote\"here");
    } else {
        panic!("Expected VectorString");
    }
}

#[test]
fn test_boolean_edge_cases() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarBool(_)));
    registry
        .register("flag".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(4).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "flag",
            vec![
                Value::ScalarBool(true),
                Value::ScalarBool(false),
                Value::ScalarBool(true),
                Value::ScalarBool(false),
            ],
        )
        .unwrap();

    if let Value::VectorBool(v) = bulk.get(&registry, "flag").unwrap() {
        assert_eq!(v, vec![true, false, true, false]);
    } else {
        panic!("Expected VectorBool");
    }

    // Partition by boolean
    let views = bulk.partition_by(&registry, "flag").unwrap();
    assert_eq!(views.len(), 2); // true and false
}

#[test]
fn test_proxy_edge_cases() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(1).unwrap();
    let bulk = bulk.set(&registry, "value", vec![Value::ScalarInt(42)]).unwrap();

    // First and only element
    let proxy = bulk.at(0).unwrap();
    assert_eq!(proxy.get_field(&registry, "value").unwrap(), Value::ScalarInt(42));
}

#[test]
fn test_version_tracking_edge_cases() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry
        .register("value".to_string(), validator, false, vec![], None)
        .unwrap();

    let bulk = Bulk::new(1).unwrap();
    assert_eq!(bulk.meta.versions.get("value"), None);

    // Set multiple times
    let mut bulk = bulk;
    for i in 1..=10 {
        bulk = bulk.set(&registry, "value", vec![Value::ScalarInt(i)]).unwrap();
        assert_eq!(bulk.meta.versions.get("value"), Some(&(i as u64)));
    }
}

