/// Comprehensive tests for derived fields, caching, and dependency chains
use soakit::*;
mod common;
use std::sync::{Arc, Mutex};

#[test]
fn test_simple_derived_field() {
    use common::fixtures::registry_with_derived;

    let registry = registry_with_derived();
    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "b",
            vec![Value::ScalarInt(5), Value::ScalarInt(15), Value::ScalarInt(25)],
        )
        .unwrap();

    // Get derived field
    let sum = bulk.get(&registry, "sum").unwrap();
    if let Value::VectorInt(v) = sum {
        assert_eq!(v, vec![15, 35, 55]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_derived_field_caching() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

    registry
        .register("a".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("b".to_string(), validator.clone(), false, vec![], None)
        .unwrap();

    // Track computation count
    let compute_count = Arc::new(Mutex::new(0));
    let compute_count_clone = compute_count.clone();
    let derived_func = Box::new(move |args: &[Value]| {
        *compute_count_clone.lock().unwrap() += 1;
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
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(10), Value::ScalarInt(20)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "b",
            vec![Value::ScalarInt(5), Value::ScalarInt(15)],
        )
        .unwrap();

    // First get - should compute
    let _sum1 = bulk.get(&registry, "sum").unwrap();
    assert_eq!(*compute_count.lock().unwrap(), 1);

    // Second get - should use cache
    let _sum2 = bulk.get(&registry, "sum").unwrap();
    assert_eq!(*compute_count.lock().unwrap(), 1); // Should still be 1
}

#[test]
fn test_cache_invalidation_on_dependency_update() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

    let compute_count = Arc::new(Mutex::new(0));
    let compute_count_clone = compute_count.clone();
    let derived_func = Box::new(move |args: &[Value]| {
        *compute_count_clone.lock().unwrap() += 1;
        if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
            let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
            Ok(Value::VectorInt(sum))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });

    registry
        .register("a".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("b".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
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
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(10), Value::ScalarInt(20)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "b",
            vec![Value::ScalarInt(5), Value::ScalarInt(15)],
        )
        .unwrap();

    // First get
    let _sum1 = bulk.get(&registry, "sum").unwrap();
    assert_eq!(*compute_count.lock().unwrap(), 1);

    // Update dependency 'a'
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(100), Value::ScalarInt(200)],
        )
        .unwrap();

    // Get again - should recompute due to cache invalidation
    let _sum2 = bulk.get(&registry, "sum").unwrap();
    assert_eq!(*compute_count.lock().unwrap(), 2);
}

#[test]
fn test_derived_field_with_three_dependencies() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

    registry
        .register("a".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("b".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("c".to_string(), validator.clone(), false, vec![], None)
        .unwrap();

    let derived_func = Box::new(|args: &[Value]| {
        if args.len() == 3 {
            if let (Value::VectorInt(a), Value::VectorInt(b), Value::VectorInt(c)) =
                (&args[0], &args[1], &args[2])
            {
                let total: Vec<i64> = a
                    .iter()
                    .zip(b.iter())
                    .zip(c.iter())
                    .map(|((x, y), z)| x + y + z)
                    .collect();
                Ok(Value::VectorInt(total))
            } else {
                Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
            }
        } else {
            Err(SoAKitError::InvalidArgument("Wrong number of args".to_string()))
        }
    });

    registry
        .register(
            "total".to_string(),
            validator,
            true,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            Some(derived_func),
        )
        .unwrap();

    let bulk = Bulk::new(2).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(1), Value::ScalarInt(2)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "b",
            vec![Value::ScalarInt(10), Value::ScalarInt(20)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "c",
            vec![Value::ScalarInt(100), Value::ScalarInt(200)],
        )
        .unwrap();

    let total = bulk.get(&registry, "total").unwrap();
    if let Value::VectorInt(v) = total {
        assert_eq!(v, vec![111, 222]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_derived_field_chain() {
    // Create a chain: a + b = sum1, sum1 + c = sum2
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

    registry
        .register("a".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("b".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("c".to_string(), validator.clone(), false, vec![], None)
        .unwrap();

    let sum1_func = Box::new(|args: &[Value]| {
        if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
            let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
            Ok(Value::VectorInt(sum))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });

    let sum2_func = Box::new(|args: &[Value]| {
        if let (Value::VectorInt(sum1), Value::VectorInt(c)) = (&args[0], &args[1]) {
            let sum: Vec<i64> = sum1.iter().zip(c.iter()).map(|(x, y)| x + y).collect();
            Ok(Value::VectorInt(sum))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });

    registry
        .register(
            "sum1".to_string(),
            validator.clone(),
            true,
            vec!["a".to_string(), "b".to_string()],
            Some(sum1_func),
        )
        .unwrap();

    registry
        .register(
            "sum2".to_string(),
            validator,
            true,
            vec!["sum1".to_string(), "c".to_string()],
            Some(sum2_func),
        )
        .unwrap();

    let bulk = Bulk::new(2).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(1), Value::ScalarInt(2)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "b",
            vec![Value::ScalarInt(10), Value::ScalarInt(20)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "c",
            vec![Value::ScalarInt(100), Value::ScalarInt(200)],
        )
        .unwrap();

    // Get sum1
    let sum1 = bulk.get(&registry, "sum1").unwrap();
    if let Value::VectorInt(v) = sum1 {
        assert_eq!(v, vec![11, 22]);
    } else {
        panic!("Expected VectorInt");
    }

    // Get sum2 (depends on sum1)
    let sum2 = bulk.get(&registry, "sum2").unwrap();
    if let Value::VectorInt(v) = sum2 {
        assert_eq!(v, vec![111, 222]);
    } else {
        panic!("Expected VectorInt");
    }
}

#[test]
fn test_cache_invalidation_in_chain() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

    let sum1_count = Arc::new(Mutex::new(0));
    let sum2_count = Arc::new(Mutex::new(0));

    let sum1_count_clone = sum1_count.clone();
    let sum1_func = Box::new(move |args: &[Value]| {
        *sum1_count_clone.lock().unwrap() += 1;
        if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
            let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
            Ok(Value::VectorInt(sum))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });

    let sum2_count_clone = sum2_count.clone();
    let sum2_func = Box::new(move |args: &[Value]| {
        *sum2_count_clone.lock().unwrap() += 1;
        if let (Value::VectorInt(sum1), Value::VectorInt(c)) = (&args[0], &args[1]) {
            let sum: Vec<i64> = sum1.iter().zip(c.iter()).map(|(x, y)| x + y).collect();
            Ok(Value::VectorInt(sum))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });

    registry
        .register("a".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("b".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("c".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register(
            "sum1".to_string(),
            validator.clone(),
            true,
            vec!["a".to_string(), "b".to_string()],
            Some(sum1_func),
        )
        .unwrap();
    registry
        .register(
            "sum2".to_string(),
            validator,
            true,
            vec!["sum1".to_string(), "c".to_string()],
            Some(sum2_func),
        )
        .unwrap();

    let bulk = Bulk::new(2).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(1), Value::ScalarInt(2)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "b",
            vec![Value::ScalarInt(10), Value::ScalarInt(20)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "c",
            vec![Value::ScalarInt(100), Value::ScalarInt(200)],
        )
        .unwrap();

    // Get sum2 (should compute both sum1 and sum2)
    let _sum2 = bulk.get(&registry, "sum2").unwrap();
    assert_eq!(*sum1_count.lock().unwrap(), 1);
    assert_eq!(*sum2_count.lock().unwrap(), 1);

    // Update 'a' - should invalidate both sum1 and sum2 caches
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(100), Value::ScalarInt(200)],
        )
        .unwrap();

    // Get sum2 again - both should recompute
    let _sum2 = bulk.get(&registry, "sum2").unwrap();
    assert_eq!(*sum1_count.lock().unwrap(), 2);
    assert_eq!(*sum2_count.lock().unwrap(), 2);
}

#[test]
fn test_derived_field_with_proxy() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

    registry
        .register("a".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("b".to_string(), validator.clone(), false, vec![], None)
        .unwrap();

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

    let bulk = Bulk::new(3).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "b",
            vec![Value::ScalarInt(5), Value::ScalarInt(15), Value::ScalarInt(25)],
        )
        .unwrap();

    // Access derived field via proxy
    let proxy = bulk.at(1).unwrap();
    let sum_value = proxy.get_field(&registry, "sum").unwrap();
    assert_eq!(sum_value, Value::ScalarInt(35)); // 20 + 15
}

#[test]
fn test_derived_field_with_view() {
    let mut registry = Registry::new();
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));

    registry
        .register("a".to_string(), validator.clone(), false, vec![], None)
        .unwrap();
    registry
        .register("b".to_string(), validator.clone(), false, vec![], None)
        .unwrap();

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

    let bulk = Bulk::new(4).unwrap();
    let bulk = bulk
        .set(
            &registry,
            "a",
            vec![
                Value::ScalarInt(10),
                Value::ScalarInt(20),
                Value::ScalarInt(30),
                Value::ScalarInt(40),
            ],
        )
        .unwrap();
    let bulk = bulk
        .set(
            &registry,
            "b",
            vec![
                Value::ScalarInt(5),
                Value::ScalarInt(15),
                Value::ScalarInt(25),
                Value::ScalarInt(35),
            ],
        )
        .unwrap();

    // Create view and access derived field
    let views = bulk.partition_by(&registry, "a").unwrap();
    // Find a view and check derived field
    if let Some(view) = views.first() {
        let sum_values = view.get_field(&registry, "sum").unwrap();
        if let Value::VectorInt(v) = sum_values {
            assert_eq!(v.len(), 1);
        } else {
            panic!("Expected VectorInt");
        }
    }
}

