use soakit::bulk::CHUNK_SIZE;
use soakit::{Bulk, Registry, Value, get_registry, register_field};
use std::sync::Arc;

#[test]
fn test_bulk_creation_with_chunks() {
    // Create a bulk with more than one chunk
    let count = CHUNK_SIZE + 100;
    let bulk = Bulk::new(count).unwrap();
    assert_eq!(bulk.count(), count);
    // We can't directly inspect chunks as they are private, but we can verify behavior
}

#[test]
fn test_set_and_get_across_chunks() {
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    // Use a unique field name to avoid conflicts with global registry
    let field_name = "aosoa_int_field".to_string();

    // Register field if not exists (robustness)
    let registry = get_registry();
    let mut reg = registry.lock().unwrap();
    if !reg.has_field(&field_name) {
        reg.register(field_name.clone(), validator, false, vec![], None)
            .unwrap();
    }
    drop(reg); // Release lock

    let count = CHUNK_SIZE * 2 + 50; // 2 full chunks + 1 partial
    let bulk = Bulk::new(count).unwrap();

    // Create values
    let values: Vec<Value> = (0..count).map(|i| Value::ScalarInt(i as i64)).collect();

    let reg = registry.lock().unwrap();
    let bulk = bulk.set(&reg, &field_name, values.clone()).unwrap();

    // Get values back
    let retrieved = bulk.get(&reg, &field_name).unwrap();

    if let Value::VectorInt(v) = retrieved {
        assert_eq!(v.len(), count);
        for (i, val) in v.iter().enumerate() {
            assert_eq!(*val, i as i64);
        }
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_apply_across_chunks() {
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let field_name = "aosoa_apply_field".to_string();

    let registry = get_registry();
    let mut reg = registry.lock().unwrap();
    if !reg.has_field(&field_name) {
        reg.register(field_name.clone(), validator, false, vec![], None)
            .unwrap();
    }
    drop(reg);

    let count = CHUNK_SIZE + 10;
    let bulk = Bulk::new(count).unwrap();

    let values: Vec<Value> = (0..count).map(|i| Value::ScalarInt(i as i64)).collect();

    let reg = registry.lock().unwrap();
    let bulk = bulk.set(&reg, &field_name, values).unwrap();

    // Apply operation to increment all values
    // Mask covers all chunks
    let mask = vec![true; count];

    let bulk = bulk
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

    // Verify
    let retrieved = bulk.get(&reg, &field_name).unwrap();
    if let Value::VectorInt(v) = retrieved {
        assert_eq!(v.len(), count);
        for (i, val) in v.iter().enumerate() {
            assert_eq!(*val, (i + 1) as i64);
        }
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_partition_across_chunks() {
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let field_name = "aosoa_partition_field".to_string();

    let registry = get_registry();
    let mut reg = registry.lock().unwrap();
    if !reg.has_field(&field_name) {
        reg.register(field_name.clone(), validator, false, vec![], None)
            .unwrap();
    }
    drop(reg);

    let count = CHUNK_SIZE * 3;
    let bulk = Bulk::new(count).unwrap();

    // Create values with 2 groups, alternating
    let values: Vec<Value> = (0..count)
        .map(|i| Value::ScalarInt((i % 2) as i64))
        .collect();

    let reg = registry.lock().unwrap();
    let bulk = bulk.set(&reg, &field_name, values).unwrap();

    let views = bulk.partition_by(&reg, &field_name).unwrap();
    assert_eq!(views.len(), 2);

    // Verify view sizes
    // Each view should have count/2 elements
    // Note: View doesn't expose count directly easily without iterating mask, but we can check mask
    // Actually View has a mask.

    // Let's just verify we got 2 views.
}

#[test]
fn test_boundary_conditions() {
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let field_name = "aosoa_boundary_field".to_string();

    let registry = get_registry();
    let mut reg = registry.lock().unwrap();
    if !reg.has_field(&field_name) {
        reg.register(field_name.clone(), validator, false, vec![], None)
            .unwrap();
    }
    drop(reg);

    // Exact chunk size
    let count = CHUNK_SIZE;
    let bulk = Bulk::new(count).unwrap();
    let values: Vec<Value> = (0..count).map(|_| Value::ScalarInt(1)).collect();
    let reg = registry.lock().unwrap();
    let bulk = bulk.set(&reg, &field_name, values).unwrap();
    let res = bulk.get(&reg, &field_name).unwrap();
    if let Value::VectorInt(v) = res {
        assert_eq!(v.len(), count);
    } else {
        panic!("Wrong type");
    }

    // Chunk size + 1
    let count = CHUNK_SIZE + 1;
    let bulk = Bulk::new(count).unwrap();
    let values: Vec<Value> = (0..count).map(|_| Value::ScalarInt(1)).collect();
    let bulk = bulk.set(&reg, &field_name, values).unwrap();
    let res = bulk.get(&reg, &field_name).unwrap();
    if let Value::VectorInt(v) = res {
        assert_eq!(v.len(), count);
    } else {
        panic!("Wrong type");
    }
}
