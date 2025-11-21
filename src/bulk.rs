/// Core Bulk data structure for SoA operations.
///
/// This module contains the [`Bulk`] structure, which is the main data container
/// in SoAKit. It implements the Structure-of-Arrays pattern, storing each field
/// as a separate array for improved cache locality and performance.
use crate::error::{Result, SoAKitError};
use crate::meta::Registry;
use crate::util::filter_system_fields;
use crate::value::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

/// Metadata for a Bulk structure.
///
/// Contains information about the bulk structure including the number of elements,
/// element IDs, and version numbers for each field. Version numbers are used for
/// cache invalidation of derived fields.
///
/// # Fields
///
/// * `count` - The number of elements in the bulk
/// * `id` - Vector of element IDs (typically 0..count-1)
/// * `versions` - Map from field names to version numbers, incremented when fields are updated
#[derive(Clone, Debug)]
pub struct Meta {
    /// Number of elements in the bulk
    pub count: usize,
    /// ID vector (0..count-1)
    pub id: Vec<usize>,
    /// Version numbers for each field, used for cache invalidation
    pub versions: BTreeMap<String, u64>,
}

impl Meta {
    /// Create new metadata for a bulk of given count.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of elements in the bulk. Must be greater than 0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Meta)` if successful, or an error if `count` is 0.
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::InvalidArgument`] if `count` is 0
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::bulk::Meta;
    ///
    /// let meta = Meta::new(10).unwrap();
    /// assert_eq!(meta.count, 10);
    /// assert_eq!(meta.id, (0..10).collect::<Vec<_>>());
    /// ```
    pub fn new(count: usize) -> Result<Self> {
        if count == 0 {
            return Err(SoAKitError::InvalidArgument(
                "Bulk count must be greater than 0".to_string(),
            ));
        }
        Ok(Self {
            count,
            id: (0..count).collect(),
            versions: BTreeMap::new(),
        })
    }
}

/// Cache entry for derived fields.
///
/// Stores a computed value along with the version numbers of its dependencies
/// at the time of computation. This allows the system to determine if the
/// cached value is still valid or needs to be recomputed.
///
/// # Fields
///
/// * `value` - The cached computed value
/// * `versions` - Version numbers of the dependencies when this value was computed
#[derive(Clone, Debug)]
pub struct CacheEntry {
    /// Cached value
    pub value: Value,
    /// Versions of dependencies when this was cached
    pub versions: Vec<u64>,
}

/// Main Bulk structure for Structure-of-Arrays operations.
///
/// The `Bulk` structure stores data using the Structure-of-Arrays (SoA) pattern,
/// where each field is stored as a separate array. This provides better cache
/// locality when processing fields independently and enables efficient vectorized
/// operations.
///
/// # Features
///
/// - **Immutable Updates**: All update operations return a new `Bulk` instance
/// - **Field Versioning**: Tracks changes to fields for cache invalidation
/// - **Derived Field Caching**: Automatically caches computed derived fields
/// - **Multiple Access Patterns**: Supports bulk operations, single element access, and views
///
/// # Fields
///
/// * `meta` - Metadata including count, IDs, and field versions
/// * `data` - Map from field names to arrays of values (one value per element)
/// * `cache` - Cache for derived field values with dependency version tracking
///
/// # Examples
///
/// ```rust
/// use soakit::{Bulk, Registry, Value};
///
/// // Create a new bulk with 3 elements
/// let bulk = Bulk::new(3).unwrap();
///
/// // Register and set a field
/// let mut registry = Registry::new();
/// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
/// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
///
/// let values = vec![
///     Value::ScalarInt(25),
///     Value::ScalarInt(30),
///     Value::ScalarInt(35),
/// ];
/// let bulk = bulk.set(&registry, "age", values).unwrap();
/// ```
#[derive(Debug)]
pub struct Bulk {
    /// Metadata (count, id, versions)
    pub meta: Meta,
    /// Field data storage: maps field names to arrays of values
    pub data: BTreeMap<String, Vec<Value>>,
    /// Cache for derived fields (using RefCell for interior mutability)
    pub cache: RefCell<BTreeMap<String, CacheEntry>>,
}

impl Bulk {
    /// Create a new Bulk structure with the given count.
    ///
    /// Creates an empty bulk structure with no field data. Fields must be
    /// registered in the registry and then set using [`Bulk::set`].
    ///
    /// # Arguments
    ///
    /// * `count` - The number of elements in the bulk. Must be greater than 0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Bulk)` if successful, or an error if `count` is 0.
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::InvalidArgument`] if `count` is 0
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Bulk;
    ///
    /// let bulk = Bulk::new(10).unwrap();
    /// assert_eq!(bulk.count(), 10);
    /// ```
    pub fn new(count: usize) -> Result<Self> {
        let meta = Meta::new(count)?;
        Ok(Self {
            meta,
            data: BTreeMap::new(),
            cache: RefCell::new(BTreeMap::new()),
        })
    }

    /// Set field values in a new Bulk (immutable update).
    ///
    /// This method creates a new `Bulk` instance with the specified field set to
    /// the provided values. The original bulk is not modified. All values must
    /// pass validation and have the same length as the bulk count.
    ///
    /// When a field is set, its version number is incremented, and any derived
    /// fields that depend on it have their cache invalidated.
    ///
    /// # Arguments
    ///
    /// * `registry` - The registry containing field metadata
    /// * `field` - The name of the field to set
    /// * `values` - A vector of values, one for each element in the bulk
    ///
    /// # Returns
    ///
    /// Returns `Ok(Bulk)` with the field set, or an error if:
    /// - The field is not registered
    /// - Validation fails
    /// - The number of values doesn't match the bulk count
    /// - Values have inconsistent lengths
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::FieldNotFound`] if the field is not registered
    /// - [`SoAKitError::ValidationFailed`] if a value fails validation
    /// - [`SoAKitError::LengthMismatch`] if the number of values doesn't match the bulk count
    /// - [`SoAKitError::InvalidArgument`] if values have inconsistent lengths
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, Registry, Value};
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let bulk = Bulk::new(3).unwrap();
    /// let values = vec![
    ///     Value::ScalarInt(25),
    ///     Value::ScalarInt(30),
    ///     Value::ScalarInt(35),
    /// ];
    /// let bulk = bulk.set(&registry, "age", values).unwrap();
    /// ```
    pub fn set(&self, registry: &Registry, field: &str, values: Vec<Value>) -> Result<Self> {
        // Validate field exists in registry
        if !registry.has_field(field) {
            return Err(SoAKitError::FieldNotFound(field.to_string()));
        }

        // Check length matches
        if values.len() != self.meta.count {
            return Err(SoAKitError::LengthMismatch {
                expected: self.meta.count,
                actual: values.len(),
            });
        }

        // Validate values (check if not empty first)
        let first_value = values
            .first()
            .ok_or_else(|| SoAKitError::InvalidArgument("Values cannot be empty".to_string()))?;
        if !registry.validate(field, first_value) {
            return Err(SoAKitError::ValidationFailed(format!(
                "Value validation failed for field: {}",
                field
            )));
        }

        // Validate all values have the same type/length
        let first_len = first_value.len();
        for (idx, val) in values.iter().enumerate() {
            if val.len() != first_len {
                return Err(SoAKitError::InvalidArgument(format!(
                    "Value at index {} has different length",
                    idx
                )));
            }
        }

        // Create new bulk with updated field
        let mut new_bulk = self.clone();
        let _ = new_bulk.data.insert(field.to_string(), values);

        // Increment version
        let current_ver = new_bulk.meta.versions.get(field).copied().unwrap_or(0);
        let new_ver = current_ver
            .checked_add(1)
            .ok_or_else(|| SoAKitError::InvalidArgument("Version overflow".to_string()))?;
        let _ = new_bulk.meta.versions.insert(field.to_string(), new_ver);

        // Invalidate cache for any derived fields that depend on this field
        new_bulk.invalidate_dependent_cache(registry, field);

        Ok(new_bulk)
    }

    /// Clone the bulk structure.
    ///
    /// Creates a deep copy of the bulk, including all field data and cache entries.
    ///
    /// # Returns
    ///
    /// A new `Bulk` instance with cloned data.
    pub fn clone(&self) -> Self {
        Self {
            meta: self.meta.clone(),
            data: self.data.clone(),
            cache: RefCell::new(self.cache.borrow().clone()),
        }
    }

    /// Get field values.
    ///
    /// Retrieves the values for a field. For regular fields, this returns the
    /// stored values. For derived fields, this computes the value (or retrieves
    /// it from cache if valid) and returns it.
    ///
    /// The returned value is always a vector type (`VectorInt`, `VectorFloat`, etc.)
    /// representing all elements' values for that field.
    ///
    /// # Arguments
    ///
    /// * `registry` - The registry containing field metadata
    /// * `field` - The name of the field to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Ok(Value)` containing the field values as a vector, or an error if:
    /// - The field is not registered
    /// - The field has no data (for regular fields)
    /// - Derived field computation fails
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::FieldNotFound`] if the field is not registered or has no data
    /// - [`SoAKitError::InvalidArgument`] if derived field computation fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, Registry, Value};
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let bulk = Bulk::new(3).unwrap();
    /// let values = vec![
    ///     Value::ScalarInt(25),
    ///     Value::ScalarInt(30),
    ///     Value::ScalarInt(35),
    /// ];
    /// let bulk = bulk.set(&registry, "age", values).unwrap();
    ///
    /// if let Value::VectorInt(ages) = bulk.get(&registry, "age").unwrap() {
    ///     assert_eq!(ages, vec![25, 30, 35]);
    /// }
    /// ```
    pub fn get(&self, registry: &Registry, field: &str) -> Result<Value> {
        let metadata = registry
            .get_metadata(field)
            .ok_or_else(|| SoAKitError::FieldNotFound(field.to_string()))?;

        if metadata.is_derived {
            // Check cache
            let cache_borrow = self.cache.borrow();
            if let Some(cache_entry) = cache_borrow.get(field) {
                // Check if dependency versions match
                let current_dep_versions: Result<Vec<u64>> = metadata
                    .dependencies
                    .iter()
                    .map(|dep| {
                        self.meta
                            .versions
                            .get(dep)
                            .copied()
                            .ok_or_else(|| SoAKitError::FieldNotFound(dep.clone()))
                    })
                    .collect();

                let current_dep_versions = current_dep_versions?;

                if cache_entry.versions == current_dep_versions {
                    return Ok(cache_entry.value.clone());
                }
            }
            drop(cache_borrow); // Release borrow before mutable borrow

            // Compute derived value
            let derived_func = metadata.derived_func.as_ref().ok_or_else(|| {
                SoAKitError::InvalidArgument("Derived field missing function".to_string())
            })?;

            // Get dependency values
            let dep_values: Result<Vec<Value>> = metadata
                .dependencies
                .iter()
                .map(|dep| self.get(registry, dep))
                .collect();

            let dep_values = dep_values?;

            // Compute derived value
            let computed_value = derived_func(&dep_values)?;

            // Get current dependency versions for caching
            let current_dep_versions: Result<Vec<u64>> = metadata
                .dependencies
                .iter()
                .map(|dep| {
                    self.meta
                        .versions
                        .get(dep)
                        .copied()
                        .ok_or_else(|| SoAKitError::FieldNotFound(dep.clone()))
                })
                .collect();

            let current_dep_versions = current_dep_versions?;

            // Update cache
            let mut cache_mut = self.cache.borrow_mut();
            let _ = cache_mut.insert(
                field.to_string(),
                CacheEntry {
                    value: computed_value.clone(),
                    versions: current_dep_versions,
                },
            );

            Ok(computed_value)
        } else {
            // Regular field - get from data
            self.data
                .get(field)
                .ok_or_else(|| SoAKitError::FieldNotFound(field.to_string()))
                .and_then(|vec_values| {
                    // Convert Vec<Value> to a single Value representing the vector
                    // In APL, getting a field returns the entire vector
                    // We need to decide: should this return Vec<Value> or a Value::Vector?
                    // Looking at the APL code, it seems like it returns the vector directly
                    // But in our Rust implementation, we store Vec<Value> where each Value is one element
                    // So we need to combine them into a single Value
                    // Actually, wait - in the APL code, each field stores a vector of values
                    // So if we have field "age" with values [10, 20, 30], we store Vec<Value> where
                    // each Value might be a scalar. But actually, looking more carefully:
                    // In APL, `bulk._data.(field)` would be a vector. So if field is "age" and
                    // we have 3 elements, `bulk._data.age` would be something like [10, 20, 30]
                    // which is a vector of 3 scalars.

                    // In our Rust implementation, we're storing Vec<Value> where each Value
                    // represents one element's value for that field. So if we have 3 elements
                    // and field "age", we might have [Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)]

                    // But the Get function should return... what? Looking at the APL code,
                    // it seems like it returns the vector directly. So we need to convert
                    // Vec<Value> into a single Value that represents a vector.

                    // However, the values might not all be the same type. Let's assume they are
                    // for now, and we can combine them into a Vector variant.

                    if vec_values.is_empty() {
                        return Err(SoAKitError::InvalidArgument(
                            "Field has no values".to_string(),
                        ));
                    }

                    // Try to combine into a single vector Value
                    // Check if all values are the same scalar type
                    let first_val = vec_values
                        .first()
                        .ok_or_else(|| SoAKitError::InvalidArgument("Empty values".to_string()))?;
                    match first_val {
                        Value::ScalarInt(_) => {
                            let ints: Result<Vec<i64>> = vec_values
                                .iter()
                                .map(|v| {
                                    if let Value::ScalarInt(i) = v {
                                        Ok(*i)
                                    } else {
                                        Err(SoAKitError::InvalidArgument(
                                            "Mixed value types in field".to_string(),
                                        ))
                                    }
                                })
                                .collect();
                            Ok(Value::VectorInt(ints?))
                        }
                        Value::ScalarFloat(_) => {
                            let floats: Result<Vec<f64>> = vec_values
                                .iter()
                                .map(|v| {
                                    if let Value::ScalarFloat(f) = v {
                                        Ok(*f)
                                    } else {
                                        Err(SoAKitError::InvalidArgument(
                                            "Mixed value types in field".to_string(),
                                        ))
                                    }
                                })
                                .collect();
                            Ok(Value::VectorFloat(floats?))
                        }
                        Value::ScalarBool(_) => {
                            let bools: Result<Vec<bool>> = vec_values
                                .iter()
                                .map(|v| {
                                    if let Value::ScalarBool(b) = v {
                                        Ok(*b)
                                    } else {
                                        Err(SoAKitError::InvalidArgument(
                                            "Mixed value types in field".to_string(),
                                        ))
                                    }
                                })
                                .collect();
                            Ok(Value::VectorBool(bools?))
                        }
                        Value::ScalarString(_) => {
                            let strings: Result<Vec<String>> = vec_values
                                .iter()
                                .map(|v| {
                                    if let Value::ScalarString(s) = v {
                                        Ok(s.clone())
                                    } else {
                                        Err(SoAKitError::InvalidArgument(
                                            "Mixed value types in field".to_string(),
                                        ))
                                    }
                                })
                                .collect();
                            Ok(Value::VectorString(strings?))
                        }
                        _ => Err(SoAKitError::InvalidArgument(
                            "Field contains non-scalar values".to_string(),
                        )),
                    }
                })
        }
    }

    /// Invalidate cache for fields that depend on the given field.
    ///
    /// When a field is updated, any derived fields that depend on it need to
    /// have their cache invalidated so they will be recomputed on the next access.
    ///
    /// # Arguments
    ///
    /// * `registry` - The registry to check for dependent fields
    /// * `field` - The name of the field that was updated
    fn invalidate_dependent_cache(&mut self, registry: &Registry, field: &str) {
        let fields_to_invalidate: Vec<String> = registry
            .list_fields()
            .into_iter()
            .filter(|f| {
                if let Some(meta) = registry.get_metadata(f) {
                    meta.is_derived && meta.dependencies.contains(&field.to_string())
                } else {
                    false
                }
            })
            .collect();

        let mut cache_mut = self.cache.borrow_mut();
        for f in fields_to_invalidate {
            let _ = cache_mut.remove(&f);
        }
    }

    /// Get the count of elements in this bulk.
    ///
    /// # Returns
    ///
    /// The number of elements in the bulk as a `usize`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Bulk;
    ///
    /// let bulk = Bulk::new(10).unwrap();
    /// assert_eq!(bulk.count(), 10);
    /// ```
    pub fn count(&self) -> usize {
        self.meta.count
    }

    /// List all data fields (excluding system fields).
    ///
    /// Returns a vector of field names that have data in this bulk.
    /// System fields (those starting with `_`) are excluded.
    ///
    /// # Returns
    ///
    /// A vector of field names as strings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, Registry, Value};
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator.clone(), false, vec![], None).unwrap();
    /// registry.register("height".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let bulk = Bulk::new(3).unwrap();
    /// let bulk = bulk.set(&registry, "age", vec![Value::ScalarInt(25); 3]).unwrap();
    /// let bulk = bulk.set(&registry, "height", vec![Value::ScalarInt(175); 3]).unwrap();
    ///
    /// let fields = bulk.list_data_fields();
    /// assert_eq!(fields.len(), 2);
    /// ```
    pub fn list_data_fields(&self) -> Vec<String> {
        filter_system_fields(&self.data.keys().cloned().collect::<Vec<_>>())
    }

    /// Create a proxy for accessing a single element at the given index.
    ///
    /// A [`Proxy`] provides a convenient way to access and manipulate a single
    /// element's field values without working with the entire bulk.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the element (0-based)
    ///
    /// # Returns
    ///
    /// Returns `Ok(Proxy)` if successful, or an error if the index is out of bounds.
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::IndexOutOfBounds`] if `idx >= bulk.count()`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, Registry, Value};
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let bulk = Bulk::new(3).unwrap();
    /// let bulk = bulk.set(&registry, "age", vec![
    ///     Value::ScalarInt(25),
    ///     Value::ScalarInt(30),
    ///     Value::ScalarInt(35),
    /// ]).unwrap();
    ///
    /// let proxy = bulk.at(1).unwrap();
    /// assert_eq!(proxy.get_field(&registry, "age").unwrap(), Value::ScalarInt(30));
    /// ```
    pub fn at(&self, idx: usize) -> Result<crate::proxy::Proxy> {
        if idx >= self.meta.count {
            return Err(SoAKitError::IndexOutOfBounds {
                index: idx,
                max: self.meta.count,
            });
        }
        crate::proxy::Proxy::new(Rc::new(self.clone()), idx)
    }

    /// Apply a function to masked subset of data.
    ///
    /// This method applies a transformation function to the values at positions
    /// where the mask is `true`, returning a new bulk with the updated values.
    /// The function receives only the masked subset of values and must return
    /// the same number of transformed values.
    ///
    /// If the mask is empty, it is treated as all `true` (applying to all elements).
    ///
    /// # Arguments
    ///
    /// * `mask` - Boolean array indicating which elements to transform (empty = all true)
    /// * `func` - Function that takes a slice of values and returns transformed values
    ///
    /// # Returns
    ///
    /// Returns `Ok(Bulk)` with updated values, or an error if:
    /// - The mask length doesn't match the bulk count (when mask is not empty)
    /// - The function returns a different number of values than masked elements
    /// - The function returns an error
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::LengthMismatch`] if mask length doesn't match or function returns wrong count
    /// - [`SoAKitError::FieldNotFound`] if a field is missing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, Registry, Value};
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let bulk = Bulk::new(5).unwrap();
    /// let bulk = bulk.set(&registry, "age", vec![
    ///     Value::ScalarInt(10),
    ///     Value::ScalarInt(20),
    ///     Value::ScalarInt(30),
    ///     Value::ScalarInt(40),
    ///     Value::ScalarInt(50),
    /// ]).unwrap();
    ///
    /// // Increment ages at positions 0, 2, 4
    /// let mask = vec![true, false, true, false, true];
    /// let new_bulk = bulk.apply(&mask, |subset| {
    ///     Ok(subset.iter().map(|v| {
    ///         if let Value::ScalarInt(i) = v {
    ///             Value::ScalarInt(i + 1)
    ///         } else {
    ///             v.clone()
    ///         }
    ///     }).collect())
    /// }).unwrap();
    /// ```
    pub fn apply<F>(&self, mask: &[bool], func: F) -> Result<Self>
    where
        F: Fn(&[Value]) -> Result<Vec<Value>>,
    {
        // Normalize mask: if empty, treat as all true
        let normalized_mask = if mask.is_empty() {
            vec![true; self.meta.count]
        } else {
            mask.to_vec()
        };

        // Validate mask length
        if normalized_mask.len() != self.meta.count {
            return Err(SoAKitError::LengthMismatch {
                expected: self.meta.count,
                actual: normalized_mask.len(),
            });
        }

        // Create new bulk
        let mut new_bulk = self.clone();

        // Get all data fields
        let fields: Vec<String> = self.data.keys().cloned().collect();

        // Update each field
        for field in fields {
            // Get old values
            let old_values = self
                .data
                .get(&field)
                .ok_or_else(|| SoAKitError::FieldNotFound(field.clone()))?;

            // Extract subset based on mask
            let subset: Vec<Value> = old_values
                .iter()
                .enumerate()
                .filter_map(|(idx, val)| {
                    if normalized_mask.get(idx).copied().unwrap_or(false) {
                        Some(val.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Apply function to subset
            let new_subset = func(&subset)?;

            // Validate new subset length matches mask count
            let mask_count = normalized_mask.iter().filter(|&&b| b).count();
            if new_subset.len() != mask_count {
                return Err(SoAKitError::LengthMismatch {
                    expected: mask_count,
                    actual: new_subset.len(),
                });
            }

            // Update values in new_bulk for masked positions
            let mut new_values = old_values.clone();
            let mut subset_idx = 0;
            for (idx, &mask_val) in normalized_mask.iter().enumerate() {
                if mask_val {
                    if let Some(new_val) = new_subset.get(subset_idx) {
                        if let Some(old_val) = new_values.get_mut(idx) {
                            *old_val = new_val.clone();
                        }
                        subset_idx = subset_idx
                            .checked_add(1)
                            .ok_or_else(|| SoAKitError::InvalidArgument("Index overflow".to_string()))?;
                    }
                }
            }

            // Update field in new bulk
            let _ = new_bulk.data.insert(field.clone(), new_values);

            // Increment version
            let current_ver = new_bulk.meta.versions.get(&field).copied().unwrap_or(0);
            let new_ver = current_ver
                .checked_add(1)
                .ok_or_else(|| SoAKitError::InvalidArgument("Version overflow".to_string()))?;
            let _ = new_bulk.meta.versions.insert(field, new_ver);
        }

        Ok(new_bulk)
    }

    /// Partition the bulk by a field's values.
    ///
    /// Creates a [`View`] for each unique value in the specified field. Each view
    /// represents a partition containing all elements that have that particular value.
    ///
    /// This is useful for grouping data by categorical values or performing
    /// operations on subsets of the data.
    ///
    /// # Arguments
    ///
    /// * `registry` - The registry containing field metadata
    /// * `field` - The name of the field to partition by
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<View>)` with one view per unique value, or an error if:
    /// - The field is not registered or has no data
    /// - The field is not a vector type
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::FieldNotFound`] if the field doesn't exist or has no data
    /// - [`SoAKitError::InvalidArgument`] if the field is not a vector
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, Registry, Value};
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("category".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let bulk = Bulk::new(6).unwrap();
    /// let bulk = bulk.set(&registry, "category", vec![
    ///     Value::ScalarInt(1),
    ///     Value::ScalarInt(2),
    ///     Value::ScalarInt(1),
    ///     Value::ScalarInt(3),
    ///     Value::ScalarInt(2),
    ///     Value::ScalarInt(1),
    /// ]).unwrap();
    ///
    /// let views = bulk.partition_by(&registry, "category").unwrap();
    /// assert_eq!(views.len(), 3); // Three unique categories
    /// ```
    pub fn partition_by(
        &self,
        registry: &Registry,
        field: &str,
    ) -> Result<Vec<crate::view::View>> {
        // Check if field exists in data
        if !self.data.contains_key(field) {
            return Err(SoAKitError::FieldNotFound(field.to_string()));
        }

        // Get field values
        let field_value = self.get(registry, field)?;

        // Extract unique values and create masks
        let (unique_values, masks) = match field_value {
            Value::VectorInt(v) => {
                let unique: Vec<i64> = v.iter().cloned().collect::<HashSet<_>>().into_iter().collect();
                let mut unique_sorted = unique;
                unique_sorted.sort();
                let masks: Vec<Vec<bool>> = unique_sorted
                    .iter()
                    .map(|&val| v.iter().map(|&x| x == val).collect())
                    .collect();
                let unique_values: Vec<Value> = unique_sorted.into_iter().map(Value::ScalarInt).collect();
                (unique_values, masks)
            }
            Value::VectorFloat(v) => {
                // For floats, we need to handle NaN and comparison carefully
                // Use a hash set with bit representation for NaN-safe comparison
                let mut seen = HashSet::new();
                let mut unique = Vec::new();
                for &val in &v {
                    // Use bit representation for NaN-safe comparison
                    let bits = f64::to_bits(val);
                    if seen.insert(bits) {
                        unique.push(val);
                    }
                }
                unique.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let masks: Vec<Vec<bool>> = unique
                    .iter()
                    .map(|&val| {
                        v.iter()
                            .map(|&x| {
                                if val.is_nan() && x.is_nan() {
                                    true
                                } else {
                                    x == val
                                }
                            })
                            .collect()
                    })
                    .collect();
                let unique_values: Vec<Value> = unique.into_iter().map(Value::ScalarFloat).collect();
                (unique_values, masks)
            }
            Value::VectorBool(v) => {
                let unique = vec![true, false];
                let masks: Vec<Vec<bool>> = unique
                    .iter()
                    .map(|&val| v.iter().map(|&x| x == val).collect())
                    .collect();
                let unique_values: Vec<Value> = unique.into_iter().map(Value::ScalarBool).collect();
                (unique_values, masks)
            }
            Value::VectorString(v) => {
                let unique: Vec<String> = v.iter().cloned().collect::<HashSet<_>>().into_iter().collect();
                let mut unique_sorted = unique;
                unique_sorted.sort();
                let masks: Vec<Vec<bool>> = unique_sorted
                    .iter()
                    .map(|val| v.iter().map(|x| x == val).collect())
                    .collect();
                let unique_values: Vec<Value> = unique_sorted.into_iter().map(Value::ScalarString).collect();
                (unique_values, masks)
            }
            _ => {
                return Err(SoAKitError::InvalidArgument(
                    "Partition field must be a vector".to_string(),
                ))
            }
        };

        // Create views
        let bulk_rc = Rc::new(self.clone());
        let views: Result<Vec<crate::view::View>> = unique_values
            .into_iter()
            .zip(masks.into_iter())
            .map(|(key, mask)| crate::view::View::new(key, mask, bulk_rc.clone()))
            .collect();

        views
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_new_bulk() {
        let bulk = Bulk::new(5).unwrap();
        assert_eq!(bulk.count(), 5);
        assert_eq!(bulk.meta.id, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_new_bulk_zero_count() {
        let result = Bulk::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_and_get() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(3).unwrap();
        let values = vec![
            Value::ScalarInt(10),
            Value::ScalarInt(20),
            Value::ScalarInt(30),
        ];
        let bulk = bulk.set(&registry, "age", values).unwrap();

        let result = bulk.get(&registry, "age").unwrap();
        if let Value::VectorInt(v) = result {
            assert_eq!(v, vec![10, 20, 30]);
        } else {
            panic!("Expected VectorInt");
        }
    }

    #[test]
    fn test_set_length_mismatch() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(3).unwrap();
        let values = vec![Value::ScalarInt(10), Value::ScalarInt(20)];
        let result = bulk.set(&registry, "age", values);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::LengthMismatch { .. }
        ));
    }

    #[test]
    fn test_set_all_value_types() {
        let mut registry = Registry::new();

        // Int field
        let int_validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), int_validator, false, vec![], None)
            .unwrap();

        // Float field
        let float_validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
        registry
            .register("height".to_string(), float_validator, false, vec![], None)
            .unwrap();

        // Bool field
        let bool_validator = Box::new(|v: &Value| matches!(v, Value::ScalarBool(_)));
        registry
            .register("active".to_string(), bool_validator, false, vec![], None)
            .unwrap();

        // String field
        let str_validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
        registry
            .register("name".to_string(), str_validator, false, vec![], None)
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
                "height",
                vec![Value::ScalarFloat(1.75), Value::ScalarFloat(1.80)],
            )
            .unwrap();
        let bulk = bulk
            .set(
                &registry,
                "active",
                vec![Value::ScalarBool(true), Value::ScalarBool(false)],
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

        // Verify all fields
        if let Value::VectorInt(v) = bulk.get(&registry, "age").unwrap() {
            assert_eq!(v, vec![25, 30]);
        } else {
            panic!("Expected VectorInt");
        }

        if let Value::VectorFloat(v) = bulk.get(&registry, "height").unwrap() {
            assert_eq!(v, vec![1.75, 1.80]);
        } else {
            panic!("Expected VectorFloat");
        }

        if let Value::VectorBool(v) = bulk.get(&registry, "active").unwrap() {
            assert_eq!(v, vec![true, false]);
        } else {
            panic!("Expected VectorBool");
        }

        if let Value::VectorString(v) = bulk.get(&registry, "name").unwrap() {
            assert_eq!(v, vec!["Alice".to_string(), "Bob".to_string()]);
        } else {
            panic!("Expected VectorString");
        }
    }

    #[test]
    fn test_version_tracking() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(3).unwrap();
        assert_eq!(bulk.meta.versions.get("age"), None);

        let bulk = bulk
            .set(
                &registry,
                "age",
                vec![
                    Value::ScalarInt(10),
                    Value::ScalarInt(20),
                    Value::ScalarInt(30),
                ],
            )
            .unwrap();
        assert_eq!(bulk.meta.versions.get("age"), Some(&1));

        let bulk = bulk
            .set(
                &registry,
                "age",
                vec![
                    Value::ScalarInt(11),
                    Value::ScalarInt(21),
                    Value::ScalarInt(31),
                ],
            )
            .unwrap();
        assert_eq!(bulk.meta.versions.get("age"), Some(&2));

        let bulk = bulk
            .set(
                &registry,
                "age",
                vec![
                    Value::ScalarInt(12),
                    Value::ScalarInt(22),
                    Value::ScalarInt(32),
                ],
            )
            .unwrap();
        assert_eq!(bulk.meta.versions.get("age"), Some(&3));
    }

    #[test]
    fn test_version_tracking_multiple_fields() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("a".to_string(), validator.clone(), false, vec![], None)
            .unwrap();
        registry
            .register("b".to_string(), validator, false, vec![], None)
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
                vec![Value::ScalarInt(3), Value::ScalarInt(4)],
            )
            .unwrap();

        assert_eq!(bulk.meta.versions.get("a"), Some(&1));
        assert_eq!(bulk.meta.versions.get("b"), Some(&1));

        let bulk = bulk
            .set(
                &registry,
                "a",
                vec![Value::ScalarInt(10), Value::ScalarInt(20)],
            )
            .unwrap();

        assert_eq!(bulk.meta.versions.get("a"), Some(&2));
        assert_eq!(bulk.meta.versions.get("b"), Some(&1));
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

        // First get should compute
        let sum1 = bulk.get(&registry, "sum").unwrap();
        if let Value::VectorInt(v) = sum1 {
            assert_eq!(v, vec![15, 35, 55]);
        } else {
            panic!("Expected VectorInt");
        }

        // Second get should use cache
        let sum2 = bulk.get(&registry, "sum").unwrap();
        if let Value::VectorInt(v) = sum2 {
            assert_eq!(v, vec![15, 35, 55]);
        } else {
            panic!("Expected VectorInt");
        }
    }

    #[test]
    fn test_cache_invalidation() {
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

        // Get sum (should compute and cache)
        let _sum1 = bulk.get(&registry, "sum").unwrap();

        // Update dependency 'a'
        let bulk = bulk
            .set(
                &registry,
                "a",
                vec![Value::ScalarInt(100), Value::ScalarInt(200)],
            )
            .unwrap();

        // Get sum again (should recompute due to cache invalidation)
        let sum2 = bulk.get(&registry, "sum").unwrap();
        if let Value::VectorInt(v) = sum2 {
            assert_eq!(v, vec![105, 215]);
        } else {
            panic!("Expected VectorInt");
        }
    }

    #[test]
    fn test_get_nonexistent_field() {
        let registry = Registry::new();
        let bulk = Bulk::new(3).unwrap();
        let result = bulk.get(&registry, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
    }

    #[test]
    fn test_set_nonexistent_field() {
        let registry = Registry::new();
        let bulk = Bulk::new(3).unwrap();
        let values = vec![
            Value::ScalarInt(10),
            Value::ScalarInt(20),
            Value::ScalarInt(30),
        ];
        let result = bulk.set(&registry, "nonexistent", values);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
    }

    #[test]
    fn test_set_validation_failure() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(3).unwrap();
        let values = vec![
            Value::ScalarFloat(10.0),
            Value::ScalarFloat(20.0),
            Value::ScalarFloat(30.0),
        ];
        let result = bulk.set(&registry, "age", values);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::ValidationFailed(_)
        ));
    }

    #[test]
    fn test_apply_operation() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(5).unwrap();
        let bulk = bulk
            .set(
                &registry,
                "age",
                vec![
                    Value::ScalarInt(10),
                    Value::ScalarInt(20),
                    Value::ScalarInt(30),
                    Value::ScalarInt(40),
                    Value::ScalarInt(50),
                ],
            )
            .unwrap();

        let mask = vec![true, false, true, false, true];
        let new_bulk = bulk
            .apply(&mask, |subset| {
                let new_vals: Vec<Value> = subset
                    .iter()
                    .map(|v| {
                        if let Value::ScalarInt(i) = v {
                            Value::ScalarInt(i + 1)
                        } else {
                            v.clone()
                        }
                    })
                    .collect();
                Ok(new_vals)
            })
            .unwrap();

        if let Value::VectorInt(v) = new_bulk.get(&registry, "age").unwrap() {
            assert_eq!(v, vec![11, 20, 31, 40, 51]);
        } else {
            panic!("Expected VectorInt");
        }
    }

    #[test]
    fn test_apply_empty_mask() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(3).unwrap();
        let bulk = bulk
            .set(
                &registry,
                "age",
                vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
            )
            .unwrap();

        // Empty mask should be treated as all true
        let new_bulk = bulk
            .apply(&[], |subset| {
                let new_vals: Vec<Value> = subset
                    .iter()
                    .map(|v| {
                        if let Value::ScalarInt(i) = v {
                            Value::ScalarInt(i + 1)
                        } else {
                            v.clone()
                        }
                    })
                    .collect();
                Ok(new_vals)
            })
            .unwrap();

        if let Value::VectorInt(v) = new_bulk.get(&registry, "age").unwrap() {
            assert_eq!(v, vec![11, 21, 31]);
        } else {
            panic!("Expected VectorInt");
        }
    }

    #[test]
    fn test_apply_mask_length_mismatch() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(3).unwrap();
        let bulk = bulk
            .set(
                &registry,
                "age",
                vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
            )
            .unwrap();

        let mask = vec![true, false]; // Wrong length
        let result = bulk.apply(&mask, |subset| Ok(subset.to_vec()));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::LengthMismatch { .. }
        ));
    }

    #[test]
    fn test_partition_by_int() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("category".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(6).unwrap();
        let bulk = bulk
            .set(
                &registry,
                "category",
                vec![
                    Value::ScalarInt(1),
                    Value::ScalarInt(2),
                    Value::ScalarInt(1),
                    Value::ScalarInt(3),
                    Value::ScalarInt(2),
                    Value::ScalarInt(1),
                ],
            )
            .unwrap();

        let views = bulk.partition_by(&registry, "category").unwrap();
        assert_eq!(views.len(), 3);

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
        assert_eq!(view_1.count(), 3);
    }

    #[test]
    fn test_partition_by_string() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
        registry
            .register("category".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(4).unwrap();
        let bulk = bulk
            .set(
                &registry,
                "category",
                vec![
                    Value::ScalarString("A".to_string()),
                    Value::ScalarString("B".to_string()),
                    Value::ScalarString("A".to_string()),
                    Value::ScalarString("C".to_string()),
                ],
            )
            .unwrap();

        let views = bulk.partition_by(&registry, "category").unwrap();
        assert_eq!(views.len(), 3);
    }

    #[test]
    fn test_partition_by_float() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
        registry
            .register("value".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(4).unwrap();
        let bulk = bulk
            .set(
                &registry,
                "value",
                vec![
                    Value::ScalarFloat(1.0),
                    Value::ScalarFloat(2.0),
                    Value::ScalarFloat(1.0),
                    Value::ScalarFloat(3.0),
                ],
            )
            .unwrap();

        let views = bulk.partition_by(&registry, "value").unwrap();
        assert_eq!(views.len(), 3);
    }

    #[test]
    fn test_partition_by_bool() {
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

        let views = bulk.partition_by(&registry, "flag").unwrap();
        assert_eq!(views.len(), 2);
    }

    #[test]
    fn test_partition_nonexistent_field() {
        let registry = Registry::new();
        let bulk = Bulk::new(3).unwrap();
        let result = bulk.partition_by(&registry, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
    }

    #[test]
    fn test_list_data_fields() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator.clone(), false, vec![], None)
            .unwrap();
        registry
            .register("height".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(3).unwrap();
        let bulk = bulk
            .set(
                &registry,
                "age",
                vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
            )
            .unwrap();
        let bulk = bulk
            .set(
                &registry,
                "height",
                vec![Value::ScalarInt(100), Value::ScalarInt(200), Value::ScalarInt(300)],
            )
            .unwrap();

        let fields = bulk.list_data_fields();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&"age".to_string()));
        assert!(fields.contains(&"height".to_string()));
    }

    #[test]
    fn test_at_proxy() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(3).unwrap();
        let bulk = bulk
            .set(
                &registry,
                "age",
                vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
            )
            .unwrap();

        let proxy = bulk.at(1).unwrap();
        assert_eq!(proxy.index(), 1);
    }

    #[test]
    fn test_at_out_of_bounds() {
        let bulk = Bulk::new(3).unwrap();
        let result = bulk.at(10);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::IndexOutOfBounds { .. }
        ));
    }

    #[test]
    fn test_single_element_bulk() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Bulk::new(1).unwrap();
        let bulk = bulk
            .set(&registry, "age", vec![Value::ScalarInt(42)])
            .unwrap();

        if let Value::VectorInt(v) = bulk.get(&registry, "age").unwrap() {
            assert_eq!(v, vec![42]);
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

        let count = 1000;
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
    fn test_meta_new() {
        let meta = Meta::new(5).unwrap();
        assert_eq!(meta.count, 5);
        assert_eq!(meta.id, vec![0, 1, 2, 3, 4]);
        assert!(meta.versions.is_empty());
    }

    #[test]
    fn test_meta_new_zero() {
        let result = Meta::new(0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::InvalidArgument(_)
        ));
    }

    #[test]
    fn test_bulk_clone() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk1 = Bulk::new(3).unwrap();
        let bulk1 = bulk1
            .set(
                &registry,
                "age",
                vec![Value::ScalarInt(10), Value::ScalarInt(20), Value::ScalarInt(30)],
            )
            .unwrap();

        let bulk2 = bulk1.clone();
        assert_eq!(bulk1.count(), bulk2.count());
        assert_eq!(
            bulk1.get(&registry, "age").unwrap(),
            bulk2.get(&registry, "age").unwrap()
        );
    }
}

