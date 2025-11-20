# Examples

Comprehensive working examples demonstrating SoAKit usage patterns.

## Example 1: Basic Field Operations

A simple example showing how to register fields, create a bulk, and set/get values.

```rust
use soakit::{init, register_field, get_registry, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Register a field
    let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    register_field("age".to_string(), validator, false, vec![], None)?;

    // Create a bulk with 3 elements
    let bulk = init(3)?;

    // Get the registry and set values
    let registry = get_registry();
    let reg = registry.lock().unwrap();
    let values = vec![
        Value::ScalarInt(25),
        Value::ScalarInt(30),
        Value::ScalarInt(35),
    ];
    let bulk = bulk.set(&reg, "age", values)?;

    // Retrieve values
    if let Value::VectorInt(ages) = bulk.get(&reg, "age")? {
        println!("Ages: {:?}", ages); // [25, 30, 35]
    }

    Ok(())
}
```

## Example 2: Multiple Fields

Working with multiple fields of different types.

```rust
use soakit::{Bulk, Registry, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();

    // Register multiple fields
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let float_validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    let bool_validator = Box::new(|v: &Value| matches!(v, Value::ScalarBool(_)));

    registry.register("age".to_string(), int_validator.clone(), false, vec![], None)?;
    registry.register("height".to_string(), float_validator, false, vec![], None)?;
    registry.register("name".to_string(), str_validator, false, vec![], None)?;
    registry.register("active".to_string(), bool_validator, false, vec![], None)?;

    // Create bulk
    let bulk = Bulk::new(3)?;

    // Set all fields
    let bulk = bulk.set(&registry, "age", vec![
        Value::ScalarInt(25),
        Value::ScalarInt(30),
        Value::ScalarInt(35),
    ])?;
    let bulk = bulk.set(&registry, "height", vec![
        Value::ScalarFloat(1.75),
        Value::ScalarFloat(1.80),
        Value::ScalarFloat(1.65),
    ])?;
    let bulk = bulk.set(&registry, "name", vec![
        Value::ScalarString("Alice".to_string()),
        Value::ScalarString("Bob".to_string()),
        Value::ScalarString("Charlie".to_string()),
    ])?;
    let bulk = bulk.set(&registry, "active", vec![
        Value::ScalarBool(true),
        Value::ScalarBool(false),
        Value::ScalarBool(true),
    ])?;

    // Retrieve and print all data
    if let Value::VectorInt(ages) = bulk.get(&registry, "age")? {
        println!("Ages: {:?}", ages);
    }
    if let Value::VectorFloat(heights) = bulk.get(&registry, "height")? {
        println!("Heights: {:?}", heights);
    }
    if let Value::VectorString(names) = bulk.get(&registry, "name")? {
        println!("Names: {:?}", names);
    }
    if let Value::VectorBool(active) = bulk.get(&registry, "active")? {
        println!("Active: {:?}", active);
    }

    Ok(())
}
```

## Example 3: Derived Fields with Caching

Creating and using derived fields that are automatically cached.

```rust
use soakit::{Bulk, Registry, Value, Result, SoAKitError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();

    // Register base fields
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry.register("a".to_string(), int_validator.clone(), false, vec![], None)?;
    registry.register("b".to_string(), int_validator.clone(), false, vec![], None)?;

    // Register derived field: sum = a + b
    let derived_validator = Box::new(|v: &Value| matches!(v, Value::VectorInt(_)));
    let sum_func = Box::new(|args: &[Value]| -> Result<Value> {
        if let (Value::VectorInt(a), Value::VectorInt(b)) = (&args[0], &args[1]) {
            let sum: Vec<i64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
            Ok(Value::VectorInt(sum))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });
    registry.register(
        "sum".to_string(),
        derived_validator,
        true,
        vec!["a".to_string(), "b".to_string()],
        Some(sum_func),
    )?;

    // Create bulk and set base fields
    let bulk = Bulk::new(3)?;
    let bulk = bulk.set(&registry, "a", vec![
        Value::ScalarInt(10),
        Value::ScalarInt(20),
        Value::ScalarInt(30),
    ])?;
    let bulk = bulk.set(&registry, "b", vec![
        Value::ScalarInt(5),
        Value::ScalarInt(15),
        Value::ScalarInt(25),
    ])?;

    // Get derived field (computed and cached)
    if let Value::VectorInt(sums) = bulk.get(&registry, "sum")? {
        println!("Sums: {:?}", sums); // [15, 35, 55]
    }

    // Get again (uses cache)
    if let Value::VectorInt(sums) = bulk.get(&registry, "sum")? {
        println!("Sums (cached): {:?}", sums); // [15, 35, 55]
    }

    // Update dependency 'a' (cache will be invalidated)
    let bulk = bulk.set(&registry, "a", vec![
        Value::ScalarInt(100),
        Value::ScalarInt(200),
        Value::ScalarInt(300),
    ])?;

    // Get derived field again (recomputed)
    if let Value::VectorInt(sums) = bulk.get(&registry, "sum")? {
        println!("Sums (recomputed): {:?}", sums); // [105, 215, 325]
    }

    Ok(())
}
```

## Example 4: Data Partitioning

Partitioning data by field values and working with views.

```rust
use soakit::{Bulk, Registry, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();

    // Register fields
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    
    registry.register("category".to_string(), int_validator.clone(), false, vec![], None)?;
    registry.register("value".to_string(), int_validator, false, vec![], None)?;
    registry.register("name".to_string(), str_validator, false, vec![], None)?;

    // Create bulk with data
    let bulk = Bulk::new(6)?;
    let bulk = bulk.set(&registry, "category", vec![
        Value::ScalarInt(1),
        Value::ScalarInt(2),
        Value::ScalarInt(1),
        Value::ScalarInt(3),
        Value::ScalarInt(2),
        Value::ScalarInt(1),
    ])?;
    let bulk = bulk.set(&registry, "value", vec![
        Value::ScalarInt(10),
        Value::ScalarInt(20),
        Value::ScalarInt(30),
        Value::ScalarInt(40),
        Value::ScalarInt(50),
        Value::ScalarInt(60),
    ])?;
    let bulk = bulk.set(&registry, "name", vec![
        Value::ScalarString("A".to_string()),
        Value::ScalarString("B".to_string()),
        Value::ScalarString("C".to_string()),
        Value::ScalarString("D".to_string()),
        Value::ScalarString("E".to_string()),
        Value::ScalarString("F".to_string()),
    ])?;

    // Partition by category
    let views = bulk.partition_by(&registry, "category")?;
    println!("Number of partitions: {}", views.len()); // 3

    // Process each partition
    for view in views {
        if let Value::ScalarInt(category) = view.key() {
            println!("\nCategory {}: {} elements", category, view.count());
            
            // Get values for this partition
            if let Value::VectorInt(values) = view.get_field(&registry, "value")? {
                println!("  Values: {:?}", values);
            }
            if let Value::VectorString(names) = view.get_field(&registry, "name")? {
                println!("  Names: {:?}", names);
            }
        }
    }

    Ok(())
}
```

## Example 5: Single Element Access

Using Proxy to access individual elements.

```rust
use soakit::{Bulk, Registry, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();

    // Register fields
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
    
    registry.register("age".to_string(), int_validator, false, vec![], None)?;
    registry.register("name".to_string(), str_validator, false, vec![], None)?;

    // Create bulk
    let bulk = Bulk::new(3)?;
    let bulk = bulk.set(&registry, "age", vec![
        Value::ScalarInt(25),
        Value::ScalarInt(30),
        Value::ScalarInt(35),
    ])?;
    let bulk = bulk.set(&registry, "name", vec![
        Value::ScalarString("Alice".to_string()),
        Value::ScalarString("Bob".to_string()),
        Value::ScalarString("Charlie".to_string()),
    ])?;

    // Access individual elements
    for i in 0..bulk.count() {
        let proxy = bulk.at(i)?;
        let age = proxy.get_field(&registry, "age")?;
        let name = proxy.get_field(&registry, "name")?;
        
        println!("Element {}: age={:?}, name={:?}", i, age, name);
    }

    Ok(())
}
```

## Example 6: Masked Operations

Applying transformations to a subset of elements using masks.

```rust
use soakit::{Bulk, Registry, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();

    // Register field
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry.register("age".to_string(), int_validator, false, vec![], None)?;

    // Create bulk
    let bulk = Bulk::new(5)?;
    let bulk = bulk.set(&registry, "age", vec![
        Value::ScalarInt(10),
        Value::ScalarInt(20),
        Value::ScalarInt(30),
        Value::ScalarInt(40),
        Value::ScalarInt(50),
    ])?;

    // Create mask: increment ages at positions 0, 2, 4
    let mask = vec![true, false, true, false, true];
    
    let new_bulk = bulk.apply(&mask, |subset| {
        Ok(subset.iter().map(|v| {
            if let Value::ScalarInt(i) = v {
                Value::ScalarInt(i + 1)
            } else {
                v.clone()
            }
        }).collect())
    })?;

    // Check results
    if let Value::VectorInt(new_ages) = new_bulk.get(&registry, "age")? {
        println!("Original: [10, 20, 30, 40, 50]");
        println!("Updated:  {:?}", new_ages); // [11, 20, 31, 40, 51]
    }

    Ok(())
}
```

## Example 7: Complex Derived Field

A more complex derived field computation.

```rust
use soakit::{Bulk, Registry, Value, Result, SoAKitError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();

    // Register base fields
    let float_validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    
    registry.register("price".to_string(), float_validator.clone(), false, vec![], None)?;
    registry.register("quantity".to_string(), int_validator, false, vec![], None)?;
    registry.register("discount".to_string(), float_validator, false, vec![], None)?;

    // Register derived field: total = (price * quantity) * (1 - discount)
    let derived_validator = Box::new(|v: &Value| matches!(v, Value::VectorFloat(_)));
    let total_func = Box::new(|args: &[Value]| -> Result<Value> {
        if let (Value::VectorFloat(prices), Value::VectorInt(quantities), Value::VectorFloat(discounts)) =
            (&args[0], &args[1], &args[2])
        {
            let totals: Vec<f64> = prices
                .iter()
                .zip(quantities.iter())
                .zip(discounts.iter())
                .map(|((p, q), d)| (p * (*q as f64)) * (1.0 - d))
                .collect();
            Ok(Value::VectorFloat(totals))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });
    registry.register(
        "total".to_string(),
        derived_validator,
        true,
        vec!["price".to_string(), "quantity".to_string(), "discount".to_string()],
        Some(total_func),
    )?;

    // Create bulk and set values
    let bulk = Bulk::new(3)?;
    let bulk = bulk.set(&registry, "price", vec![
        Value::ScalarFloat(10.0),
        Value::ScalarFloat(20.0),
        Value::ScalarFloat(30.0),
    ])?;
    let bulk = bulk.set(&registry, "quantity", vec![
        Value::ScalarInt(2),
        Value::ScalarInt(3),
        Value::ScalarInt(4),
    ])?;
    let bulk = bulk.set(&registry, "discount", vec![
        Value::ScalarFloat(0.1),  // 10% discount
        Value::ScalarFloat(0.2),  // 20% discount
        Value::ScalarFloat(0.15), // 15% discount
    ])?;

    // Get computed totals
    if let Value::VectorFloat(totals) = bulk.get(&registry, "total")? {
        println!("Totals: {:?}", totals);
        // [18.0, 48.0, 102.0]
        // (10*2*0.9, 20*3*0.8, 30*4*0.85)
    }

    Ok(())
}
```

## Example 8: Custom Validators

Using custom validation logic.

```rust
use soakit::{Bulk, Registry, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();

    // Custom validator: age must be between 0 and 150
    let age_validator = Box::new(|v: &Value| {
        if let Value::ScalarInt(age) = v {
            *age >= 0 && *age <= 150
        } else {
            false
        }
    });
    registry.register("age".to_string(), age_validator, false, vec![], None)?;

    // Create bulk
    let bulk = Bulk::new(3)?;

    // Valid values
    let result = bulk.set(&registry, "age", vec![
        Value::ScalarInt(25),
        Value::ScalarInt(30),
        Value::ScalarInt(35),
    ]);
    assert!(result.is_ok());

    // Invalid value (age > 150)
    let result = bulk.set(&registry, "age", vec![
        Value::ScalarInt(25),
        Value::ScalarInt(200), // Invalid!
        Value::ScalarInt(35),
    ]);
    assert!(result.is_err()); // Validation failed

    Ok(())
}
```

## Example 9: Working with Views

Advanced view operations.

```rust
use soakit::{Bulk, Registry, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();

    // Register fields
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    registry.register("department".to_string(), int_validator.clone(), false, vec![], None)?;
    registry.register("salary".to_string(), int_validator, false, vec![], None)?;

    // Create employee data
    let bulk = Bulk::new(10)?;
    let bulk = bulk.set(&registry, "department", vec![
        Value::ScalarInt(1), Value::ScalarInt(2), Value::ScalarInt(1),
        Value::ScalarInt(3), Value::ScalarInt(2), Value::ScalarInt(1),
        Value::ScalarInt(3), Value::ScalarInt(2), Value::ScalarInt(1),
        Value::ScalarInt(3),
    ])?;
    let bulk = bulk.set(&registry, "salary", vec![
        Value::ScalarInt(50000), Value::ScalarInt(60000), Value::ScalarInt(55000),
        Value::ScalarInt(70000), Value::ScalarInt(65000), Value::ScalarInt(52000),
        Value::ScalarInt(75000), Value::ScalarInt(68000), Value::ScalarInt(58000),
        Value::ScalarInt(72000),
    ])?;

    // Partition by department
    let views = bulk.partition_by(&registry, "department")?;

    // Calculate average salary per department
    for view in views {
        if let Value::ScalarInt(dept) = view.key() {
            if let Value::VectorInt(salaries) = view.get_field(&registry, "salary")? {
                let sum: i64 = salaries.iter().sum();
                let avg = sum as f64 / salaries.len() as f64;
                println!("Department {}: {} employees, avg salary: {:.2}", 
                    dept, view.count(), avg);
            }
        }
    }

    Ok(())
}
```

## Example 10: Complete Workflow

A complete example showing a typical workflow.

```rust
use soakit::{Bulk, Registry, Value, Result, SoAKitError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create registry and register fields
    let mut registry = Registry::new();
    
    let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    let float_validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
    
    registry.register("id".to_string(), int_validator.clone(), false, vec![], None)?;
    registry.register("price".to_string(), float_validator.clone(), false, vec![], None)?;
    registry.register("quantity".to_string(), int_validator.clone(), false, vec![], None)?;
    registry.register("tax_rate".to_string(), float_validator.clone(), false, vec![], None)?;

    // Register derived field: subtotal = price * quantity
    let subtotal_func = Box::new(|args: &[Value]| -> Result<Value> {
        if let (Value::VectorFloat(prices), Value::VectorInt(quantities)) = (&args[0], &args[1]) {
            let subtotals: Vec<f64> = prices.iter()
                .zip(quantities.iter())
                .map(|(p, q)| p * (*q as f64))
                .collect();
            Ok(Value::VectorFloat(subtotals))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });
    registry.register(
        "subtotal".to_string(),
        Box::new(|v: &Value| matches!(v, Value::VectorFloat(_))),
        true,
        vec!["price".to_string(), "quantity".to_string()],
        Some(subtotal_func),
    )?;

    // Register derived field: total = subtotal * (1 + tax_rate)
    let total_func = Box::new(|args: &[Value]| -> Result<Value> {
        if let (Value::VectorFloat(subtotals), Value::VectorFloat(tax_rates)) = (&args[0], &args[1]) {
            let totals: Vec<f64> = subtotals.iter()
                .zip(tax_rates.iter())
                .map(|(s, t)| s * (1.0 + t))
                .collect();
            Ok(Value::VectorFloat(totals))
        } else {
            Err(SoAKitError::InvalidArgument("Invalid arguments".to_string()))
        }
    });
    registry.register(
        "total".to_string(),
        Box::new(|v: &Value| matches!(v, Value::VectorFloat(_))),
        true,
        vec!["subtotal".to_string(), "tax_rate".to_string()],
        Some(total_func),
    )?;

    // 2. Create bulk and populate data
    let bulk = Bulk::new(4)?;
    let bulk = bulk.set(&registry, "id", vec![
        Value::ScalarInt(1),
        Value::ScalarInt(2),
        Value::ScalarInt(3),
        Value::ScalarInt(4),
    ])?;
    let bulk = bulk.set(&registry, "price", vec![
        Value::ScalarFloat(10.0),
        Value::ScalarFloat(20.0),
        Value::ScalarFloat(15.0),
        Value::ScalarFloat(25.0),
    ])?;
    let bulk = bulk.set(&registry, "quantity", vec![
        Value::ScalarInt(2),
        Value::ScalarInt(3),
        Value::ScalarInt(1),
        Value::ScalarInt(4),
    ])?;
    let bulk = bulk.set(&registry, "tax_rate", vec![
        Value::ScalarFloat(0.1),  // 10%
        Value::ScalarFloat(0.15), // 15%
        Value::ScalarFloat(0.1),  // 10%
        Value::ScalarFloat(0.2),  // 20%
    ])?;

    // 3. Access computed fields
    if let Value::VectorFloat(subtotals) = bulk.get(&registry, "subtotal")? {
        println!("Subtotals: {:?}", subtotals);
    }
    if let Value::VectorFloat(totals) = bulk.get(&registry, "total")? {
        println!("Totals: {:?}", totals);
    }

    // 4. Access individual elements
    for i in 0..bulk.count() {
        let proxy = bulk.at(i)?;
        let id = proxy.get_field(&registry, "id")?;
        let total = proxy.get_field(&registry, "total")?;
        println!("Item {:?}: total = {:?}", id, total);
    }

    Ok(())
}
```

These examples demonstrate various usage patterns and can be used as starting points for your own applications.

