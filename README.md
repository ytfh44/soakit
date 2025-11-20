# SoAKit - Structure-of-Arrays Kit

A high-performance Rust library for managing structured data using the Structure-of-Arrays (SoA) pattern. SoAKit provides field metadata management, derived fields with automatic caching, versioning, and multiple access patterns for efficient data manipulation.

## Features

- **Structure-of-Arrays Pattern**: Store each field as a separate array for better cache locality and vectorized operations
- **Field Metadata System**: Register fields with validators and type information
- **Derived Fields**: Compute fields from other fields with automatic caching and cache invalidation
- **Versioning**: Track changes to fields for efficient cache management
- **Multiple Access Patterns**: 
  - Bulk operations on entire fields
  - Single element access via `Proxy`
  - Partitioned views via `View`
- **Type Safety**: Strong typing with runtime validation
- **Immutable Updates**: All updates return new instances, enabling safe concurrent access patterns

## Installation

Add SoAKit to your `Cargo.toml`:

```toml
[dependencies]
soakit = "0.1.0"
```

## Quick Start

```rust
use soakit::{init, register_field, get_registry, Value};

// Register a field
let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
register_field("age".to_string(), validator, false, vec![], None).unwrap();

// Create a bulk structure with 3 elements
let bulk = init(3).unwrap();

// Get the registry and set values
let registry = get_registry();
let reg = registry.lock().unwrap();
let values = vec![
    Value::ScalarInt(25),
    Value::ScalarInt(30),
    Value::ScalarInt(35),
];
let bulk = bulk.set(&reg, "age", values).unwrap();

// Retrieve values
if let Value::VectorInt(ages) = bulk.get(&reg, "age").unwrap() {
    println!("Ages: {:?}", ages); // [25, 30, 35]
}
```

## Documentation

- **[Architecture Guide](docs/architecture.md)**: System design and core concepts
- **[API Reference](docs/api.md)**: Complete API documentation
- **[Usage Guide](docs/usage.md)**: Common patterns and best practices
- **[Examples](docs/examples.md)**: Comprehensive working examples

## Basic Usage

### Registering Fields

```rust
use soakit::{register_field, Value};

// Register a regular field
let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
register_field("age".to_string(), validator, false, vec![], None).unwrap();

// Register a derived field
let validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
let derived_func = Box::new(|args: &[Value]| {
    if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
        let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
        Ok(Value::VectorInt(sum))
    } else {
        Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
    }
});
register_field(
    "sum".to_string(),
    validator,
    true,
    vec!["a".to_string(), "b".to_string()],
    Some(derived_func),
).unwrap();
```

### Working with Bulk Data

```rust
use soakit::{Bulk, Registry, Value};

let mut registry = Registry::new();
let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
registry.register("age".to_string(), validator, false, vec![], None).unwrap();

// Create bulk and set values
let bulk = Bulk::new(3).unwrap();
let bulk = bulk.set(&registry, "age", vec![
    Value::ScalarInt(25),
    Value::ScalarInt(30),
    Value::ScalarInt(35),
]).unwrap();

// Get values
if let Value::VectorInt(ages) = bulk.get(&registry, "age").unwrap() {
    println!("Ages: {:?}", ages);
}
```

### Single Element Access

```rust
// Access a single element
let proxy = bulk.at(1).unwrap();
let age = proxy.get_field(&registry, "age").unwrap();
println!("Element 1 age: {:?}", age);
```

### Partitioning Data

```rust
// Partition by a field's values
let views = bulk.partition_by(&registry, "category").unwrap();
for view in views {
    println!("Partition key: {:?}, count: {}", view.key(), view.count());
}
```

## Project Structure

```
soakit/
├── src/
│   ├── lib.rs          # Main library entry point
│   ├── bulk.rs         # Core Bulk data structure
│   ├── value.rs        # Value types (scalars, vectors, matrices)
│   ├── meta.rs         # Field metadata and registry
│   ├── view.rs         # Partitioned data views
│   ├── proxy.rs        # Single element access
│   ├── error.rs        # Error types
│   └── util.rs         # Utility functions
├── tests/              # Integration tests
└── docs/               # Documentation
```

## Requirements

- Rust 1.75.0 or later
- No external dependencies (uses only standard library)

## License

This project is licensed under the same terms as Rust itself.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## See Also

- [Architecture Documentation](docs/architecture.md)
- [API Reference](docs/api.md)
- [Usage Guide](docs/usage.md)
- [Examples](docs/examples.md)

