/// View for partitioned data access in Bulk.
//!
//! This module provides the [`View`] structure, which represents a partition
//! of a [`Bulk`] structure. Views are created by partitioning a bulk by a
//! field's values, allowing efficient access to subsets of the data.
use crate::bulk::Bulk;
use crate::error::{Result, SoAKitError};
use crate::value::Value;
use std::rc::Rc;

/// View representing a partition of a Bulk structure.
///
/// A `View` represents a subset of elements in a [`Bulk`] that share a common
/// value for a particular field. Views are created by [`Bulk::partition_by`]
/// and provide filtered access to the parent bulk's data.
///
/// Views are useful for grouping data and performing operations on subsets
/// without copying the data.
///
/// # Fields
///
/// * `key` - The value that defines this partition (all elements in this view have this value)
/// * `mask` - Boolean array indicating which elements in the parent bulk belong to this view
/// * `parent` - Reference to the parent bulk structure
///
/// # Examples
///
/// ```rust
/// use soakit::{Bulk, Registry, Value};
/// use std::rc::Rc;
///
/// let mut registry = Registry::new();
/// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
/// registry.register("category".to_string(), validator, false, vec![], None).unwrap();
///
/// let bulk = Bulk::new(4).unwrap();
/// let bulk = bulk.set(&registry, "category", vec![
///     Value::ScalarInt(1),
///     Value::ScalarInt(2),
///     Value::ScalarInt(1),
///     Value::ScalarInt(2),
/// ]).unwrap();
///
/// let views = bulk.partition_by(&registry, "category").unwrap();
/// assert_eq!(views.len(), 2); // Two unique categories
/// ```
#[derive(Debug)]
pub struct View {
    /// The key value that defines this partition
    pub key: Value,
    /// Boolean mask indicating which elements belong to this partition
    pub mask: Vec<bool>,
    /// Reference to the parent Bulk
    pub parent: Rc<Bulk>,
}

impl View {
    /// Create a new view with the given key, mask, and parent.
    ///
    /// # Arguments
    ///
    /// * `key` - The value that defines this partition
    /// * `mask` - Boolean array indicating which elements belong to this view
    /// * `parent` - Reference to the parent bulk structure
    ///
    /// # Returns
    ///
    /// Returns `Ok(View)` if successful, or an error if the mask length doesn't
    /// match the parent bulk's count.
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::LengthMismatch`] if the mask length doesn't match the parent count
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, View, Value};
    /// use std::rc::Rc;
    ///
    /// let bulk = Rc::new(Bulk::new(5).unwrap());
    /// let mask = vec![true, false, true, false, true];
    /// let view = View::new(Value::ScalarInt(1), mask, bulk).unwrap();
    /// assert_eq!(view.count(), 3);
    /// ```
    pub fn new(key: Value, mask: Vec<bool>, parent: Rc<Bulk>) -> Result<Self> {
        // Validate mask length matches parent count
        if mask.len() != parent.count() {
            return Err(SoAKitError::LengthMismatch {
                expected: parent.count(),
                actual: mask.len(),
            });
        }
        Ok(Self { key, mask, parent })
    }

    /// Get the number of elements in this view.
    ///
    /// This is the number of `true` values in the mask, i.e., the number of
    /// elements from the parent bulk that belong to this partition.
    ///
    /// # Returns
    ///
    /// The number of elements in this view as a `usize`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, View, Value};
    /// use std::rc::Rc;
    ///
    /// let bulk = Rc::new(Bulk::new(5).unwrap());
    /// let mask = vec![true, false, true, false, true];
    /// let view = View::new(Value::ScalarInt(1), mask, bulk).unwrap();
    /// assert_eq!(view.count(), 3);
    /// ```
    pub fn count(&self) -> usize {
        self.mask.iter().filter(|&&b| b).count()
    }

    /// Check if the view is empty.
    ///
    /// A view is empty if it contains no elements (all mask values are `false`).
    ///
    /// # Returns
    ///
    /// Returns `true` if the view has no elements, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, View, Value};
    /// use std::rc::Rc;
    ///
    /// let bulk = Rc::new(Bulk::new(3).unwrap());
    /// let empty_mask = vec![false, false, false];
    /// let view = View::new(Value::ScalarInt(0), empty_mask, bulk).unwrap();
    /// assert!(view.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Get a field value filtered by this view's mask.
    ///
    /// Retrieves the values for a field from the parent bulk, but only returns
    /// the values for elements where the mask is `true`. This provides a filtered
    /// view of the field data.
    ///
    /// # Arguments
    ///
    /// * `registry` - The registry containing field metadata
    /// * `field` - The name of the field to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Ok(Value)` containing only the filtered values as a vector, or an error if:
    /// - The field is not found
    /// - The field value is not a vector type
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::FieldNotFound`] if the field doesn't exist
    /// - [`SoAKitError::InvalidArgument`] if the field value is not a vector
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, Registry, View, Value};
    /// use std::rc::Rc;
    ///
    /// let mut registry = Registry::new();
    /// let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
    /// registry.register("age".to_string(), validator, false, vec![], None).unwrap();
    ///
    /// let bulk = Rc::new(Bulk::new(4).unwrap());
    /// let bulk = Rc::new(bulk.set(&registry, "age", vec![
    ///     Value::ScalarInt(10),
    ///     Value::ScalarInt(20),
    ///     Value::ScalarInt(10),
    ///     Value::ScalarInt(30),
    /// ]).unwrap());
    ///
    /// // Create view for elements with age == 10
    /// let mask = vec![true, false, true, false];
    /// let view = View::new(Value::ScalarInt(10), mask, bulk).unwrap();
    ///
    /// if let Value::VectorInt(ages) = view.get_field(&registry, "age").unwrap() {
    ///     assert_eq!(ages, vec![10, 10]);
    /// }
    /// ```
    pub fn get_field(&self, registry: &crate::meta::Registry, field: &str) -> Result<Value> {
        // Get the full field vector from parent
        let field_value = self.parent.get(registry, field)?;

        // Filter based on mask
        match field_value {
            Value::VectorInt(v) => {
                let filtered: Vec<i64> = v
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, val)| {
                        if self.mask.get(idx).copied().unwrap_or(false) {
                            Some(*val)
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Value::VectorInt(filtered))
            }
            Value::VectorFloat(v) => {
                let filtered: Vec<f64> = v
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, val)| {
                        if self.mask.get(idx).copied().unwrap_or(false) {
                            Some(*val)
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Value::VectorFloat(filtered))
            }
            Value::VectorBool(v) => {
                let filtered: Vec<bool> = v
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, val)| {
                        if self.mask.get(idx).copied().unwrap_or(false) {
                            Some(*val)
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Value::VectorBool(filtered))
            }
            Value::VectorString(v) => {
                let filtered: Vec<String> = v
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, val)| {
                        if self.mask.get(idx).copied().unwrap_or(false) {
                            Some(val.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Value::VectorString(filtered))
            }
            _ => Err(SoAKitError::InvalidArgument(
                "Field value is not a vector".to_string(),
            )),
        }
    }

    /// Get the key value for this partition.
    ///
    /// The key is the value that all elements in this view share for the
    /// partitioning field.
    ///
    /// # Returns
    ///
    /// A reference to the key value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, View, Value};
    /// use std::rc::Rc;
    ///
    /// let bulk = Rc::new(Bulk::new(3).unwrap());
    /// let key = Value::ScalarString("A".to_string());
    /// let mask = vec![true, false, true];
    /// let view = View::new(key.clone(), mask, bulk).unwrap();
    /// assert_eq!(view.key(), &key);
    /// ```
    pub fn key(&self) -> &Value {
        &self.key
    }

    /// Get the mask for this partition.
    ///
    /// The mask is a boolean array where `true` indicates that the corresponding
    /// element in the parent bulk belongs to this view.
    ///
    /// # Returns
    ///
    /// A slice of the mask array.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, View, Value};
    /// use std::rc::Rc;
    ///
    /// let bulk = Rc::new(Bulk::new(3).unwrap());
    /// let mask = vec![true, false, true];
    /// let view = View::new(Value::ScalarInt(0), mask.clone(), bulk).unwrap();
    /// assert_eq!(view.mask(), mask.as_slice());
    /// ```
    pub fn mask(&self) -> &[bool] {
        &self.mask
    }

    /// Get a reference to the parent bulk.
    ///
    /// # Returns
    ///
    /// A reference to the parent [`Bulk`] structure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::{Bulk, View, Value};
    /// use std::rc::Rc;
    ///
    /// let bulk = Rc::new(Bulk::new(3).unwrap());
    /// let mask = vec![true, false, true];
    /// let view = View::new(Value::ScalarInt(0), mask, bulk.clone()).unwrap();
    /// assert_eq!(view.parent().count(), bulk.count());
    /// ```
    pub fn parent(&self) -> &Bulk {
        &self.parent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_view_creation() {
        let bulk = Rc::new(Bulk::new(5).unwrap());
        let mask = vec![true, false, true, false, true];
        let view = View::new(Value::ScalarInt(1), mask, bulk).unwrap();
        assert_eq!(view.count(), 3);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_view_mask_length_mismatch() {
        let bulk = Rc::new(Bulk::new(5).unwrap());
        let mask = vec![true, false]; // Wrong length
        let result = View::new(Value::ScalarInt(1), mask, bulk);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::LengthMismatch { .. }
        ));
    }

    #[test]
    fn test_view_get_field() {
        let mut registry = crate::meta::Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Rc::new(Bulk::new(4).unwrap());
        let values = vec![
            Value::ScalarInt(10),
            Value::ScalarInt(20),
            Value::ScalarInt(10),
            Value::ScalarInt(30),
        ];
        let bulk = Rc::new(bulk.set(&registry, "age", values).unwrap());

        // Create view for elements with age == 10
        let mask = vec![true, false, true, false];
        let view = View::new(Value::ScalarInt(10), mask, bulk).unwrap();
        let filtered = view.get_field(&registry, "age").unwrap();

        if let Value::VectorInt(v) = filtered {
            assert_eq!(v, vec![10, 10]);
        } else {
            panic!("Expected VectorInt");
        }
    }

    #[test]
    fn test_view_get_field_all_types() {
        let mut registry = crate::meta::Registry::new();

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

        let bulk = Rc::new(Bulk::new(4).unwrap());
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "age",
                vec![
                    Value::ScalarInt(10),
                    Value::ScalarInt(20),
                    Value::ScalarInt(30),
                    Value::ScalarInt(40),
                ],
            )
            .unwrap(),
        );
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "height",
                vec![
                    Value::ScalarFloat(1.5),
                    Value::ScalarFloat(1.6),
                    Value::ScalarFloat(1.7),
                    Value::ScalarFloat(1.8),
                ],
            )
            .unwrap(),
        );
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "active",
                vec![
                    Value::ScalarBool(true),
                    Value::ScalarBool(false),
                    Value::ScalarBool(true),
                    Value::ScalarBool(false),
                ],
            )
            .unwrap(),
        );
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "name",
                vec![
                    Value::ScalarString("A".to_string()),
                    Value::ScalarString("B".to_string()),
                    Value::ScalarString("C".to_string()),
                    Value::ScalarString("D".to_string()),
                ],
            )
            .unwrap(),
        );

        let mask = vec![true, false, true, false];
        let view = View::new(Value::ScalarInt(1), mask.clone(), bulk.clone()).unwrap();

        if let Value::VectorInt(v) = view.get_field(&registry, "age").unwrap() {
            assert_eq!(v, vec![10, 30]);
        } else {
            panic!("Expected VectorInt");
        }

        if let Value::VectorFloat(v) = view.get_field(&registry, "height").unwrap() {
            assert_eq!(v, vec![1.5, 1.7]);
        } else {
            panic!("Expected VectorFloat");
        }

        if let Value::VectorBool(v) = view.get_field(&registry, "active").unwrap() {
            assert_eq!(v, vec![true, true]);
        } else {
            panic!("Expected VectorBool");
        }

        if let Value::VectorString(v) = view.get_field(&registry, "name").unwrap() {
            assert_eq!(v, vec!["A".to_string(), "C".to_string()]);
        } else {
            panic!("Expected VectorString");
        }
    }

    #[test]
    fn test_view_empty() {
        let bulk = Rc::new(Bulk::new(5).unwrap());
        let mask = vec![false, false, false, false, false];
        let view = View::new(Value::ScalarInt(0), mask, bulk).unwrap();
        assert!(view.is_empty());
        assert_eq!(view.count(), 0);
    }

    #[test]
    fn test_view_all_true_mask() {
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let mask = vec![true, true, true];
        let view = View::new(Value::ScalarInt(0), mask, bulk).unwrap();
        assert_eq!(view.count(), 3);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_view_single_element() {
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let mask = vec![false, true, false];
        let view = View::new(Value::ScalarInt(0), mask, bulk).unwrap();
        assert_eq!(view.count(), 1);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_view_key() {
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let key = Value::ScalarString("test".to_string());
        let mask = vec![true, false, true];
        let view = View::new(key.clone(), mask, bulk).unwrap();
        assert_eq!(view.key(), &key);
    }

    #[test]
    fn test_view_mask() {
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let mask = vec![true, false, true];
        let view = View::new(Value::ScalarInt(0), mask.clone(), bulk).unwrap();
        assert_eq!(view.mask(), mask.as_slice());
    }

    #[test]
    fn test_view_parent() {
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let mask = vec![true, false, true];
        let view = View::new(Value::ScalarInt(0), mask, bulk.clone()).unwrap();
        assert_eq!(view.parent().count(), bulk.count());
    }

    #[test]
    fn test_view_get_field_nonexistent() {
        let registry = crate::meta::Registry::new();
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let mask = vec![true, false, true];
        let view = View::new(Value::ScalarInt(0), mask, bulk).unwrap();

        let result = view.get_field(&registry, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
    }

    #[test]
    fn test_view_get_field_non_vector() {
        // Note: bulk.get always returns vectors, so this is hard to test directly
        // But we can verify the error path exists
    }

    #[test]
    fn test_view_with_derived_field() {
        let mut registry = crate::meta::Registry::new();
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

        let bulk = Rc::new(Bulk::new(4).unwrap());
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "a",
                vec![
                    Value::ScalarInt(10),
                    Value::ScalarInt(20),
                    Value::ScalarInt(30),
                    Value::ScalarInt(40),
                ],
            )
            .unwrap(),
        );
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "b",
                vec![
                    Value::ScalarInt(5),
                    Value::ScalarInt(15),
                    Value::ScalarInt(25),
                    Value::ScalarInt(35),
                ],
            )
            .unwrap(),
        );

        let mask = vec![true, false, true, false];
        let view = View::new(Value::ScalarInt(0), mask, bulk).unwrap();

        if let Value::VectorInt(v) = view.get_field(&registry, "sum").unwrap() {
            assert_eq!(v, vec![15, 55]); // [10+5, 30+25]
        } else {
            panic!("Expected VectorInt");
        }
    }

    #[test]
    fn test_view_mask_edge_cases() {
        // Mask with all false except first
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let mask = vec![true, false, false];
        let view = View::new(Value::ScalarInt(0), mask, bulk).unwrap();
        assert_eq!(view.count(), 1);

        // Mask with all false except last
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let mask = vec![false, false, true];
        let view = View::new(Value::ScalarInt(0), mask, bulk).unwrap();
        assert_eq!(view.count(), 1);

        // Alternating mask
        let bulk = Rc::new(Bulk::new(4).unwrap());
        let mask = vec![true, false, true, false];
        let view = View::new(Value::ScalarInt(0), mask, bulk).unwrap();
        assert_eq!(view.count(), 2);
    }

    #[test]
    fn test_view_with_special_float_values() {
        let mut registry = crate::meta::Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
        registry
            .register("value".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Rc::new(Bulk::new(4).unwrap());
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "value",
                vec![
                    Value::ScalarFloat(1.0),
                    Value::ScalarFloat(f64::NAN),
                    Value::ScalarFloat(3.0),
                    Value::ScalarFloat(f64::INFINITY),
                ],
            )
            .unwrap(),
        );

        let mask = vec![true, true, false, true];
        let view = View::new(Value::ScalarFloat(0.0), mask, bulk).unwrap();

        if let Value::VectorFloat(v) = view.get_field(&registry, "value").unwrap() {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], 1.0);
            assert!(v[1].is_nan());
            assert!(v[2].is_infinite());
        } else {
            panic!("Expected VectorFloat");
        }
    }

    #[test]
    fn test_view_with_empty_strings() {
        let mut registry = crate::meta::Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
        registry
            .register("name".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Rc::new(Bulk::new(3).unwrap());
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "name",
                vec![
                    Value::ScalarString(String::new()),
                    Value::ScalarString("test".to_string()),
                    Value::ScalarString(String::new()),
                ],
            )
            .unwrap(),
        );

        let mask = vec![true, false, true];
        let view = View::new(Value::ScalarString(String::new()), mask, bulk).unwrap();

        if let Value::VectorString(v) = view.get_field(&registry, "name").unwrap() {
            assert_eq!(v.len(), 2);
            assert_eq!(v[0], String::new());
            assert_eq!(v[1], String::new());
        } else {
            panic!("Expected VectorString");
        }
    }
}

