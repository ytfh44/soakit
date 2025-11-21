# Architecture Documentation

## Overview

SoAKit implements a Structure-of-Arrays (SoA) data management system in Rust. Instead of storing data as an Array-of-Structs (AoS), where each element is a struct containing all fields, SoAKit stores each field as a separate array. This design provides better cache locality when processing fields independently and enables efficient vectorized operations.

## Core Concepts

### Structure-of-Arrays Pattern

In the SoA pattern, data is organized by field rather than by element:

**Array-of-Structs (AoS)** - Traditional approach:
```
Element 0: {age: 25, height: 1.75, name: "Alice"}
Element 1: {age: 30, height: 1.80, name: "Bob"}
Element 2: {age: 35, height: 1.65, name: "Charlie"}
```

**Structure-of-Arrays (SoA)** - SoAKit approach:
```
age:    [25, 30, 35]
height: [1.75, 1.80, 1.65]
name:   ["Alice", "Bob", "Charlie"]
```

### Benefits

1. **Cache Locality**: When processing a single field, all values are contiguous in memory
2. **Vectorization**: Operations on entire fields can be vectorized more easily
3. **Memory Efficiency**: Only load the fields you need into cache
4. **Parallel Processing**: Different fields can be processed in parallel

## System Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────┐
│                      Application                        │
└────────────────────┬────────────────────────────────────┘
                     │
         ┌───────────┴───────────┐
         │                       │
    ┌────▼────┐            ┌─────▼─────┐
    │ Registry│            │   Bulk    │
    │         │            │           │
    │ - Fields│            │ - Data    │
    │ - Valid │            │ - Meta    │
    │ - Derived│           │ - Cache   │
    └────┬────┘            └─────┬─────┘
         │                       │
         │                       │
    ┌────▼───────────────────────▼────┐
    │         Access Patterns          │
    │                                  │
    │  ┌──────┐  ┌──────┐  ┌──────┐  │
    │  │ Bulk │  │ Proxy│  │ View │  │
    │  │ Ops  │  │      │  │      │  │
    │  └──────┘  └──────┘  └──────┘  │
    └─────────────────────────────────┘
```

### Core Components

#### 1. Registry

The `Registry` stores metadata for all fields that can be used in `Bulk` structures:

- **Field Definitions**: Name, validator function, type information
- **Derived Field Metadata**: Dependencies and computation functions
- **Validation**: Ensures values match field requirements

**Key Responsibilities:**
- Field registration and validation
- Metadata lookup
- Dependency tracking for derived fields

#### 2. Bulk

The `Bulk` structure is the main data container:

- **Data Storage**: Maps field names to arrays of values
- **Metadata**: Element count, IDs, and field versions
- **Cache**: Stores computed derived field values

**Key Responsibilities:**
- Storing and retrieving field data
- Managing field versions for cache invalidation
- Computing derived fields with caching
- Providing access patterns (bulk, proxy, view)

#### 3. Value Types

The `Value` enum represents all possible data types:

- **Scalars**: Single values (Int, Float, Bool, String)
- **Vectors**: 1D arrays of primitives
- **Matrices**: Nested structures (2D+)

**Key Responsibilities:**
- Type representation and checking
- Shape and rank information
- Element extraction

#### 4. Access Patterns

**Bulk Operations**: Direct access to entire fields
```rust
let ages = bulk.get(&registry, "age")?;
```

**Proxy**: Single element access
```rust
let proxy = bulk.at(1)?;
let age = proxy.get_field(&registry, "age")?;
```

**View**: Partitioned access
```rust
let views = bulk.partition_by(&registry, "category")?;
```

## Data Flow

### Setting Field Values

```
1. User calls bulk.set(registry, field, values)
   │
   ├─► Validate field exists in registry
   ├─► Validate all values pass field validator
   ├─► Check value count matches bulk count
   │
   ├─► Create new Bulk with updated field
   ├─► Increment field version
   └─► Invalidate cache for dependent derived fields
```

### Getting Field Values

```
1. User calls bulk.get(registry, field)
   │
   ├─► Check if field is derived
   │   │
   │   ├─► [Regular Field]
   │   │   └─► Return stored values
   │   │
   │   └─► [Derived Field]
   │       ├─► Check cache
   │       │   ├─► [Cache Hit] Check dependency versions
   │       │   │   ├─► [Valid] Return cached value
   │       │   │   └─► [Invalid] Recompute
   │       │   │
   │       │   └─► [Cache Miss] Recompute
   │       │
   │       ├─► Get dependency values (recursive)
   │       ├─► Compute derived value
   │       └─► Update cache with new value and versions
```

### Derived Field Computation

```
1. Derived field requested
   │
   ├─► Get dependency field values (recursive get)
   ├─► Call derived function with dependencies
   ├─► Validate computed value
   ├─► Store in cache with dependency versions
   └─► Return computed value
```

## Design Decisions

### Immutable Updates

All update operations return new `Bulk` instances rather than modifying in place:

**Rationale:**
- Enables safe concurrent access patterns
- Simplifies reasoning about data flow
- Allows for efficient sharing of unchanged data (via `Rc`)

**Trade-offs:**
- Slightly higher memory usage (mitigated by `Rc` sharing)
- More allocations (acceptable for typical use cases)

### Version-Based Cache Invalidation

Derived fields cache their computed values along with the versions of their dependencies:

**Rationale:**
- Efficient: Only recompute when dependencies actually change
- Automatic: No manual cache management required
- Correct: Ensures cache consistency

**Implementation:**
- Each field has a version number incremented on update
- Cache entries store dependency versions
- Cache is valid if stored versions match current versions

### Global Registry

The library provides a global registry singleton:

**Rationale:**
- Convenient for many use cases
- Fields are typically registered once at startup
- Thread-safe via `Mutex`

**Alternative:**
- Users can create their own `Registry` instances for isolation

### RefCell for Cache

The cache uses `RefCell` for interior mutability:

**Rationale:**
- Allows cache updates during immutable `get` operations
- Maintains `Bulk` as an immutable data structure
- Safe because cache is internal implementation detail

## Performance Considerations

### Cache Locality

SoA layout provides excellent cache locality when:
- Processing a single field across all elements
- Performing vectorized operations
- Filtering or transforming field data

### Memory Layout

```
Bulk {
  meta: Meta { ... },
  data: {
    "age": [25, 30, 35, ...],      // Contiguous
    "height": [1.75, 1.80, ...],   // Contiguous
    "name": ["Alice", "Bob", ...]  // Contiguous
  }
}
```

### Cache Efficiency

- Derived fields are computed once and cached
- Cache invalidation is O(dependent_fields) on field update
- Cache lookup is O(1) with version check O(dependencies)

## Extension Points

### Custom Validators

Users can provide custom validation logic:

```rust
let validator = Box::new(|v: &Value| {
    if let Value::ScalarInt(i) = v {
        *i >= 0 && *i <= 150  // Age validation
    } else {
        false
    }
});
```

### Custom Derived Functions

Users can define complex derived field computations:

```rust
let derived_func = Box::new(|args: &[Value]| {
    // Custom computation logic
    Ok(computed_value)
});
```

## Future Enhancements

For a comprehensive overview of planned features, implementation phases, and development priorities, see the [Development Roadmap](roadmap.md).

The roadmap organizes planned enhancements into six implementation phases:

1. **Foundation & Core Infrastructure**: Serialization, basic querying, type system enhancements, memory management
2. **Query & Data Operations**: Advanced querying, transformations, data cleaning, joins
3. **Performance & Scalability**: Parallel processing, indexing, lazy evaluation, memory optimization
4. **Concurrency & Advanced Features**: Thread safety, async support, transactions, time series
5. **Ecosystem & Tooling**: Plugin system, external integrations, CLI/REPL, monitoring, ML integration
6. **Specialized Domains**: Graph data support, advanced ML features, WebAssembly optimization

Each phase builds upon previous phases, ensuring foundational capabilities are established before advanced features are implemented.

