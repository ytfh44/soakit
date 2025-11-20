# Usage Guide

This guide covers common usage patterns, best practices, and practical examples for using SoAKit.

## Getting Started

### Basic Workflow

1. **Register fields** in the registry
2. **Create a Bulk** structure with the desired number of elements
3. **Set field values** for your data
4. **Access and manipulate** data using various patterns

### Example: Simple Data Management

```rust
use soakit::{init, register_field, get_registry, Value};

// 1. Register fields
let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
register_field("age".to_string(), validator, false, vec![], None).unwrap();

// 2. Create bulk
let bulk = init(3).unwrap();

// 3. Set values
let registry = get_registry();
let reg = registry.lock().unwrap();
let bulk = bulk.set(&reg, "age", vec![
    Value::ScalarInt(25),
    Value::ScalarInt(30),
    Value::ScalarInt(35),
]).unwrap();

// 4. Access data
if let Value::VectorInt(ages) = bulk.get(&reg, "age").unwrap() {
    println!("Ages: {:?}", ages);
}
```

## Field Registration

### Regular Fields

Regular fields store data directly and are not computed from other fields.

```rust
use soakit::{register_field, Value};

// Integer field
let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
register_field("age".to_string(), int_validator, false, vec![], None).unwrap();

// Float field
let float_validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
register_field("height".to_string(), float_validator, false, vec![], None).unwrap();

// String field
let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
register_field("name".to_string(), str_validator, false, vec![], None).unwrap();

// Boolean field
let bool_validator = Box::new(|v: &Value| matches!(v, Value::ScalarBool(_)));
register_field("active".to_string(), bool_validator, false, vec![], None).unwrap();
```

### Custom Validators

You can provide custom validation logic:

```rust
// Age must be between 0 and 150
let age_validator = Box::new(|v: &Value| {
    if let Value::ScalarInt(age) = v {
        *age >= 0 && *age <= 150
    } else {
        false
    }
});
register_field("age".to_string(), age_validator, false, vec![], None).unwrap();

// Height must be positive
let height_validator = Box::new(|v: &Value| {
    if let Value::ScalarFloat(h) = v {
        *h > 0.0
    } else {
        false
    }
});
register_field("height".to_string(), height_validator, false, vec![], None).unwrap();
```

### Derived Fields

Derived fields are computed from other fields and automatically cached.

```rust
use soakit::{register_field, Value, Result, SoAKitError};

// Register base fields
let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
register_field("a".to_string(), int_validator.clone(), false, vec![], None).unwrap();
register_field("b".to_string(), int_validator.clone(), false, vec![], None).unwrap();

// Register derived field: sum = a + b
let derived_validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
let sum_func = Box::new(|args: &[Value]| {
    if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
        let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
        Ok(Value::VectorInt(sum))
    } else {
        Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
    }
});
register_field(
    "sum".to_string(),
    derived_validator,
    true,
    vec!["a".to_string(), "b".to_string()],
    Some(sum_func),
).unwrap();
```

## Working with Bulk Data

### Setting Multiple Fields

```rust
use soakit::{Bulk, Registry, Value};

let mut registry = Registry::new();
let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));

registry.register("age".to_string(), int_validator.clone(), false, vec![], None).unwrap();
registry.register("height".to_string(), int_validator, false, vec![], None).unwrap();
registry.register("name".to_string(), str_validator, false, vec![], None).unwrap();

let bulk = Bulk::new(3).unwrap();
let bulk = bulk.set(&registry, "age", vec![
    Value::ScalarInt(25),
    Value::ScalarInt(30),
    Value::ScalarInt(35),
]).unwrap();
let bulk = bulk.set(&registry, "height", vec![
    Value::ScalarInt(175),
    Value::ScalarInt(180),
    Value::ScalarInt(165),
]).unwrap();
let bulk = bulk.set(&registry, "name", vec![
    Value::ScalarString("Alice".to_string()),
    Value::ScalarString("Bob".to_string()),
    Value::ScalarString("Charlie".to_string()),
]).unwrap();
```

### Retrieving Field Values

```rust
// Get all ages
if let Value::VectorInt(ages) = bulk.get(&registry, "age").unwrap() {
    println!("All ages: {:?}", ages);
}

// Get derived field (automatically computed and cached)
if let Value::VectorInt(sums) = bulk.get(&registry, "sum").unwrap() {
    println!("Sums: {:?}", sums);
}
```

## Access Patterns

### Bulk Operations

Direct access to entire fields:

```rust
// Get entire field
let ages = bulk.get(&registry, "age").unwrap();

// Apply transformation to masked subset
let mask = vec![true, false, true, false, true];
let new_bulk = bulk.apply(&mask, |subset| {
    Ok(subset.iter().map(|v| {
        if let Value::ScalarInt(i) = v {
            Value::ScalarInt(i + 1)
        } else {
            v.clone()
        }
    }).collect())
}).unwrap();
```

### Single Element Access (Proxy)

Access individual elements:

```rust
// Get a proxy for element at index 1
let proxy = bulk.at(1).unwrap();

// Access fields for this element
let age = proxy.get_field(&registry, "age").unwrap();
let name = proxy.get_field(&registry, "name").unwrap();

println!("Element 1: age={:?}, name={:?}", age, name);
```

### Partitioned Views

Group data by field values:

```rust
// Partition by category
let views = bulk.partition_by(&registry, "category").unwrap();

for view in views {
    println!("Category: {:?}, Count: {}", view.key(), view.count());
    
    // Get filtered data for this partition
    if let Value::VectorInt(ages) = view.get_field(&registry, "age").unwrap() {
        println!("  Ages in this category: {:?}", ages);
    }
}
```

## Common Patterns

### Pattern 1: Data Processing Pipeline

```rust
// 1. Register fields
let mut registry = Registry::new();
// ... register fields ...

// 2. Create and populate bulk
let mut bulk = Bulk::new(1000).unwrap();
// ... set field values ...

// 3. Process data
let views = bulk.partition_by(&registry, "category").unwrap();
for view in views {
    // Process each partition
    let data = view.get_field(&registry, "value").unwrap();
    // ... process data ...
}
```

### Pattern 2: Computed Metrics

```rust
// Register base fields
registry.register("price".to_string(), ...).unwrap();
registry.register("quantity".to_string(), ...).unwrap();

// Register computed field: total = price * quantity
let total_func = Box::new(|args: &[Value]| {
    if let (Value::VectorFloat(prices), Value::VectorInt(quantities)) = (&args[0], &args[1]) {
        let totals: Vec<f64> = prices.iter()
            .zip(quantities.iter())
            .map(|(p, q)| p * (*q as f64))
            .collect();
        Ok(Value::VectorFloat(totals))
    } else {
        Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
    }
});
registry.register(
    "total".to_string(),
    validator,
    true,
    vec!["price".to_string(), "quantity".to_string()],
    Some(total_func),
).unwrap();

// Use computed field (automatically cached)
let totals = bulk.get(&registry, "total").unwrap();
```

### Pattern 3: Filtering and Transformation

```rust
// Filter elements where age > 30
let ages = bulk.get(&registry, "age").unwrap();
if let Value::VectorInt(age_vec) = ages {
    let mask: Vec<bool> = age_vec.iter().map(|&age| age > 30).collect();
    
    // Apply transformation only to filtered elements
    let new_bulk = bulk.apply(&mask, |subset| {
        // Transform subset
        Ok(subset.iter().map(|v| {
            // ... transformation logic ...
            v.clone()
        }).collect())
    }).unwrap();
}
```

## Best Practices

### 1. Field Registration

- **Register all fields before use**: Fields must be registered before they can be used in a Bulk
- **Use descriptive names**: Choose clear, descriptive field names
- **Validate early**: Use validators to catch invalid data early
- **Group related fields**: Register related fields together for clarity

### 2. Error Handling

Always handle errors appropriately:

```rust
match bulk.set(&registry, "age", values) {
    Ok(new_bulk) => {
        // Success
    }
    Err(SoAKitError::FieldNotFound(field)) => {
        eprintln!("Field {} not found", field);
    }
    Err(SoAKitError::ValidationFailed(msg)) => {
        eprintln!("Validation failed: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

### 3. Derived Fields

- **Keep dependencies minimal**: Derived fields with fewer dependencies are more efficient
- **Use caching effectively**: Derived fields are automatically cached, so repeated access is fast
- **Handle errors in derived functions**: Always return appropriate errors from derived functions

### 4. Performance

- **Batch operations**: When possible, set multiple fields in sequence rather than one at a time
- **Use views for filtering**: Views are efficient for accessing filtered subsets
- **Leverage caching**: Derived fields are cached automatically, so repeated access is fast

### 5. Memory Management

- **Immutable updates**: Remember that updates return new Bulk instances
- **Reuse registries**: Create one registry and reuse it
- **Consider Rc sharing**: Bulk structures use Rc internally for efficient sharing

## Performance Considerations

### Cache Locality

SoA layout provides excellent cache locality when:
- Processing a single field across all elements
- Performing vectorized operations
- Filtering or transforming field data

### Derived Field Caching

Derived fields are automatically cached:
- First access computes and caches the value
- Subsequent accesses use the cached value (if dependencies haven't changed)
- Cache is invalidated when dependencies are updated

### Memory Usage

- Each field is stored as a separate array
- Derived fields store computed values in cache
- Views and proxies share references to parent bulk (via Rc)

## Common Pitfalls

### 1. Forgetting to Register Fields

Always register fields before using them:

```rust
// ❌ Wrong: Field not registered
let bulk = bulk.set(&registry, "age", values).unwrap(); // Error!

// ✅ Correct: Register first
registry.register("age".to_string(), validator, false, vec![], None).unwrap();
let bulk = bulk.set(&registry, "age", values).unwrap();
```

### 2. Length Mismatch

Ensure value count matches bulk count:

```rust
// ❌ Wrong: Wrong number of values
let bulk = Bulk::new(5).unwrap();
let values = vec![Value::ScalarInt(1), Value::ScalarInt(2)]; // Only 2 values!
let bulk = bulk.set(&registry, "age", values).unwrap(); // Error!

// ✅ Correct: Match count
let bulk = Bulk::new(5).unwrap();
let values = vec![Value::ScalarInt(1); 5]; // 5 values
let bulk = bulk.set(&registry, "age", values).unwrap();
```

### 3. Invalid Field Names

Field names must not start with underscore:

```rust
// ❌ Wrong: Starts with underscore
register_field("_internal".to_string(), validator, false, vec![], None).unwrap(); // Error!

// ✅ Correct: Valid name
register_field("internal".to_string(), validator, false, vec![], None).unwrap();
```

### 4. Derived Field Dependencies

Derived fields must have dependencies:

```rust
// ❌ Wrong: No dependencies
register_field("sum".to_string(), validator, true, vec![], None).unwrap(); // Error!

// ✅ Correct: Has dependencies
register_field(
    "sum".to_string(),
    validator,
    true,
    vec!["a".to_string(), "b".to_string()],
    Some(derived_func),
).unwrap();
```

## Advanced Usage

### Custom Registry

You can create your own registry instance instead of using the global one:

```rust
use soakit::Registry;

let mut registry = Registry::new();
// Register fields in your registry
// Use this registry with your Bulk instances
```

### Complex Derived Fields

Derived fields can have complex computation logic:

```rust
let complex_func = Box::new(|args: &[Value]| {
    // Complex computation involving multiple dependencies
    // ... your logic here ...
    Ok(computed_value)
});
```

### Working with Large Datasets

For large datasets:
- Process in batches using views
- Use derived fields to precompute expensive operations
- Leverage caching to avoid redundant computations

