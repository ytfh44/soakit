# API Reference

Complete API documentation for SoAKit, organized by module.

## Table of Contents

- [Core Library](#core-library)
- [Value Types](#value-types)
- [Bulk Operations](#bulk-operations)
- [Metadata Registry](#metadata-registry)
- [View](#view)
- [Proxy](#proxy)
- [Error Types](#error-types)
- [Utilities](#utilities)

## Core Library

### Functions

#### `init(count: usize) -> Result<Bulk>`

Initialize a new Bulk structure with the specified number of elements.

**Parameters:**
- `count`: The number of elements in the bulk (must be > 0)

**Returns:**
- `Ok(Bulk)` if successful
- `Err(SoAKitError::InvalidArgument)` if count is 0

**Example:**
```rust
let bulk = init(10).unwrap();
```

#### `get_registry() -> &'static Mutex<Registry>`

Get or initialize the global registry instance.

**Returns:**
- A reference to the thread-safe global registry

**Example:**
```rust
let registry = get_registry();
let reg = registry.lock().unwrap();
```

#### `register_field(...) -> Result<()>`

Register a field in the global registry.

**Parameters:**
- `name`: Field name (must not start with `_` and not be empty)
- `validator`: Function that validates values for this field
- `is_derived`: Whether this is a derived field
- `dependencies`: For derived fields, the names of fields this depends on
- `derived_func`: For derived fields, the computation function

**Returns:**
- `Ok(())` if successful
- `Err(SoAKitError::InvalidArgument)` if name is invalid or arguments inconsistent
- `Err(SoAKitError::FieldAlreadyExists)` if field already exists
- `Err(SoAKitError::DerivedFieldNoDeps)` if derived field has no dependencies

**Example:**
```rust
let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
register_field("age".to_string(), validator, false, vec![], None).unwrap();
```

## Value Types

### Enum `Value`

Represents all possible data types in SoAKit.

**Variants:**
- `ScalarInt(i64)`: 64-bit signed integer
- `ScalarFloat(f64)`: 64-bit floating-point number
- `ScalarBool(bool)`: Boolean value
- `ScalarString(String)`: String value
- `VectorInt(Vec<i64>)`: Vector of integers
- `VectorFloat(Vec<f64>)`: Vector of floats
- `VectorBool(Vec<bool>)`: Vector of booleans
- `VectorString(Vec<String>)`: Vector of strings
- `Matrix(Vec<Value>)`: Matrix (nested structures)

### Methods

#### `is_scalar(&self) -> bool`

Check if the value is a scalar (rank 0).

#### `is_vector(&self) -> bool`

Check if the value is a vector (rank 1).

#### `is_matrix(&self) -> bool`

Check if the value is a matrix (rank 2+).

#### `rank(&self) -> usize`

Get the rank (number of dimensions) of the value.
- Returns `0` for scalars
- Returns `1` for vectors
- Returns `2` for matrices

#### `len(&self) -> usize`

Get the length of the value.
- Scalars: always `1`
- Vectors: number of elements
- Matrices: number of rows

#### `is_empty(&self) -> bool`

Check if the value is empty (length 0).

#### `shape(&self) -> Vec<usize>`

Get the shape (dimensions) of the value.
- Scalars: `[]`
- Vectors: `[length]`
- Matrices: `[rows, columns]`

#### `get_element(&self, idx: usize) -> Result<Value>`

Extract a single element from a vector by index.

**Parameters:**
- `idx`: The index of the element (0-based)

**Returns:**
- `Ok(Value)` containing the scalar value at the index
- `Err(SoAKitError::IndexOutOfBounds)` if index is out of bounds
- `Err(SoAKitError::InvalidArgument)` if value is not a vector

## Bulk Operations

### Struct `Bulk`

Main data structure for SoA operations.

**Fields:**
- `meta: Meta`: Metadata (count, IDs, versions)
- `data: BTreeMap<String, Vec<Value>>`: Field data storage
- `cache: RefCell<BTreeMap<String, CacheEntry>>`: Cache for derived fields

### Methods

#### `new(count: usize) -> Result<Bulk>`

Create a new Bulk structure with the given count.

**Parameters:**
- `count`: Number of elements (must be > 0)

**Returns:**
- `Ok(Bulk)` if successful
- `Err(SoAKitError::InvalidArgument)` if count is 0

#### `set(&self, registry: &Registry, field: &str, values: Vec<Value>) -> Result<Bulk>`

Set field values in a new Bulk (immutable update).

**Parameters:**
- `registry`: The registry containing field metadata
- `field`: The name of the field to set
- `values`: A vector of values, one for each element

**Returns:**
- `Ok(Bulk)` with the field set
- `Err(SoAKitError::FieldNotFound)` if field is not registered
- `Err(SoAKitError::ValidationFailed)` if validation fails
- `Err(SoAKitError::LengthMismatch)` if value count doesn't match bulk count

#### `get(&self, registry: &Registry, field: &str) -> Result<Value>`

Get field values. Handles both regular and derived fields with caching.

**Parameters:**
- `registry`: The registry containing field metadata
- `field`: The name of the field to retrieve

**Returns:**
- `Ok(Value)` containing the field values as a vector
- `Err(SoAKitError::FieldNotFound)` if field doesn't exist
- `Err(SoAKitError::InvalidArgument)` if derived field computation fails

#### `count(&self) -> usize`

Get the count of elements in this bulk.

#### `at(&self, idx: usize) -> Result<Proxy>`

Create a proxy for accessing a single element at the given index.

**Parameters:**
- `idx`: The index of the element (0-based)

**Returns:**
- `Ok(Proxy)` if successful
- `Err(SoAKitError::IndexOutOfBounds)` if index is out of bounds

#### `apply<F>(&self, mask: &[bool], func: F) -> Result<Bulk>`

Apply a function to masked subset of data.

**Parameters:**
- `mask`: Boolean array indicating which elements to transform (empty = all true)
- `func`: Function that takes a slice of values and returns transformed values

**Returns:**
- `Ok(Bulk)` with updated values
- `Err(SoAKitError::LengthMismatch)` if mask length doesn't match or function returns wrong count

#### `partition_by(&self, registry: &Registry, field: &str) -> Result<Vec<View>>`

Partition the bulk by a field's values.

**Parameters:**
- `registry`: The registry containing field metadata
- `field`: The name of the field to partition by

**Returns:**
- `Ok(Vec<View>)` with one view per unique value
- `Err(SoAKitError::FieldNotFound)` if field doesn't exist
- `Err(SoAKitError::InvalidArgument)` if field is not a vector

#### `list_data_fields(&self) -> Vec<String>`

List all data fields (excluding system fields).

### Struct `Meta`

Metadata for a Bulk structure.

**Fields:**
- `count: usize`: Number of elements
- `id: Vec<usize>`: Element IDs (typically 0..count-1)
- `versions: BTreeMap<String, u64>`: Version numbers for each field

### Methods

#### `new(count: usize) -> Result<Meta>`

Create new metadata for a bulk of given count.

### Struct `CacheEntry`

Cache entry for derived fields.

**Fields:**
- `value: Value`: Cached value
- `versions: Vec<u64>`: Versions of dependencies when this was cached

## Metadata Registry

### Struct `Registry`

Registry for field metadata.

### Methods

#### `new() -> Registry`

Create a new empty registry.

#### `register(...) -> Result<()>`

Register a new field.

**Parameters:**
- `name`: Field name
- `validator`: Validator function
- `is_derived`: Whether this is a derived field
- `dependencies`: For derived fields, dependency names
- `derived_func`: For derived fields, computation function

**Returns:**
- `Ok(())` if successful
- Various errors for invalid inputs

#### `validate(&self, field: &str, value: &Value) -> bool`

Validate a value against a field's validator.

**Returns:**
- `true` if valid, `false` otherwise

#### `get_metadata(&self, field: &str) -> Option<&FieldMetadata>`

Get metadata for a field.

#### `has_field(&self, field: &str) -> bool`

Check if a field exists in the registry.

#### `list_fields(&self) -> Vec<String>`

List all registered field names.

#### `len(&self) -> usize`

Get the number of registered fields.

#### `is_empty(&self) -> bool`

Check if the registry is empty.

### Struct `FieldMetadata`

Metadata for a field in the registry.

**Fields:**
- `validator: Box<dyn Fn(&Value) -> bool + Send + Sync>`
- `is_derived: bool`
- `dependencies: Vec<String>`
- `derived_func: Option<Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>>`

### Methods

#### `new(validator: ...) -> FieldMetadata`

Create metadata for a regular field.

#### `new_derived(validator: ..., dependencies: ..., derived_func: ...) -> Result<FieldMetadata>`

Create metadata for a derived field.

## View

### Struct `View`

View representing a partition of a Bulk structure.

**Fields:**
- `key: Value`: The key value that defines this partition
- `mask: Vec<bool>`: Boolean mask indicating which elements belong to this partition
- `parent: Rc<Bulk>`: Reference to the parent Bulk

### Methods

#### `new(key: Value, mask: Vec<bool>, parent: Rc<Bulk>) -> Result<View>`

Create a new view.

**Returns:**
- `Ok(View)` if successful
- `Err(SoAKitError::LengthMismatch)` if mask length doesn't match parent count

#### `count(&self) -> usize`

Get the number of elements in this view.

#### `is_empty(&self) -> bool`

Check if the view is empty.

#### `get_field(&self, registry: &Registry, field: &str) -> Result<Value>`

Get a field value filtered by this view's mask.

**Returns:**
- `Ok(Value)` containing only the filtered values
- `Err(SoAKitError::FieldNotFound)` if field doesn't exist
- `Err(SoAKitError::InvalidArgument)` if field value is not a vector

#### `key(&self) -> &Value`

Get the key value for this partition.

#### `mask(&self) -> &[bool]`

Get the mask for this partition.

#### `parent(&self) -> &Bulk`

Get a reference to the parent bulk.

## Proxy

### Struct `Proxy`

Proxy for accessing a single element in a Bulk structure.

**Fields:**
- `bulk: Rc<Bulk>`: Reference to the parent Bulk
- `idx: usize`: Index of the element

### Methods

#### `new(bulk: Rc<Bulk>, idx: usize) -> Result<Proxy>`

Create a new proxy.

**Returns:**
- `Ok(Proxy)` if successful
- `Err(SoAKitError::IndexOutOfBounds)` if index is out of bounds

#### `get_field(&self, registry: &Registry, field: &str) -> Result<Value>`

Get a field value for this element (returns scalar value).

**Returns:**
- `Ok(Value)` containing the scalar value for this element
- `Err(SoAKitError::FieldNotFound)` if field doesn't exist
- `Err(SoAKitError::InvalidArgument)` if field value is not a vector

#### `index(&self) -> usize`

Get the index this proxy represents.

#### `bulk(&self) -> &Bulk`

Get a reference to the parent bulk.

## Error Types

### Enum `SoAKitError`

Main error type for SoAKit operations.

**Variants:**
- `InvalidArgument(String)`: Invalid function argument
- `FieldNotFound(String)`: Field not found in registry
- `ValidationFailed(String)`: Field validation failed
- `IndexOutOfBounds { index: usize, max: usize }`: Index out of bounds
- `LengthMismatch { expected: usize, actual: usize }`: Value length doesn't match bulk count
- `DerivedFieldNoDeps(String)`: Derived field missing dependencies
- `FieldAlreadyExists(String)`: Field already registered

### Type `Result<T>`

Type alias: `std::result::Result<T, SoAKitError>`

## Utilities

### Functions

#### `is_scalar(value: &Value) -> bool`

Check if a value is a scalar (rank 0).

#### `is_vector(value: &Value) -> bool`

Check if a value is a vector (rank 1).

#### `is_matrix(value: &Value) -> bool`

Check if a value is a matrix (rank 2+).

#### `is_valid_field_name(name: &str) -> bool`

Validate a field name. Field names must not start with underscore and must not be empty.

#### `filter_system_fields(names: &[String]) -> Vec<String>`

Filter out system/internal field names (those starting with underscore).

