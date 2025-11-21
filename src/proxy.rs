/// Proxy for single element access in Bulk.
///
/// This module provides the [`Proxy`] structure, which provides a convenient
/// way to access and manipulate a single element's field values in a [`Bulk`]
/// structure without working with the entire bulk.
use crate::bulk::Bulk;
use crate::error::{Result, SoAKitError};
use crate::meta::Registry;
use crate::value::Value;
use std::rc::Rc;

/// Proxy for accessing a single element in a Bulk structure.
///
/// A `Proxy` provides a convenient interface for accessing a single element's
/// field values. When you call `get_field` on a proxy, it returns scalar values
/// (not vectors) representing that single element's data.
///
/// Proxies are created using [`Bulk::at`] and provide an object-oriented
/// access pattern for individual elements.
///
/// # Fields
///
/// * `bulk` - Reference to the parent bulk structure
/// * `idx` - Index of the element this proxy represents
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
#[derive(Debug)]
pub struct Proxy {
    /// Reference to the parent Bulk
    bulk: Rc<Bulk>,
    /// Index of the element this proxy represents
    idx: usize,
}

impl Proxy {
    /// Create a new proxy for the given bulk and index.
    ///
    /// # Arguments
    ///
    /// * `bulk` - Reference to the parent bulk structure
    /// * `idx` - Index of the element (0-based, must be < bulk.count())
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
    /// use soakit::{Bulk, Proxy};
    /// use std::rc::Rc;
    ///
    /// let bulk = Rc::new(Bulk::new(5).unwrap());
    /// let proxy = Proxy::new(bulk.clone(), 2).unwrap();
    /// assert_eq!(proxy.index(), 2);
    ///
    /// // Out of bounds
    /// assert!(Proxy::new(bulk, 10).is_err());
    /// ```
    pub fn new(bulk: Rc<Bulk>, idx: usize) -> Result<Self> {
        if idx >= bulk.count() {
            return Err(SoAKitError::IndexOutOfBounds {
                index: idx,
                max: bulk.count(),
            });
        }
        Ok(Self { bulk, idx })
    }

    /// Get a field value for this element.
    ///
    /// Retrieves the value of a field for the single element this proxy represents.
    /// Unlike [`Bulk::get`], which returns a vector of all elements' values, this
    /// method returns a scalar value for just this element.
    ///
    /// # Arguments
    ///
    /// * `registry` - The registry containing field metadata
    /// * `field` - The name of the field to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Ok(Value)` containing the scalar value for this element, or an error if:
    /// - The field is not found
    /// - The field value is not a vector type
    /// - The index is out of bounds (shouldn't happen if proxy was created correctly)
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::FieldNotFound`] if the field doesn't exist
    /// - [`SoAKitError::InvalidArgument`] if the field value is not a vector
    /// - [`SoAKitError::IndexOutOfBounds`] if the index is out of bounds
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
    pub fn get_field(&self, registry: &Registry, field: &str) -> Result<Value> {
        // Get the full field vector
        let field_value = self.bulk.get(registry, field)?;

        // Extract the element at our index
        match field_value {
            Value::VectorInt(v) => v.get(self.idx).copied().map(Value::ScalarInt).ok_or(
                SoAKitError::IndexOutOfBounds {
                    index: self.idx,
                    max: v.len(),
                },
            ),
            Value::VectorFloat(v) => v.get(self.idx).copied().map(Value::ScalarFloat).ok_or(
                SoAKitError::IndexOutOfBounds {
                    index: self.idx,
                    max: v.len(),
                },
            ),
            Value::VectorBool(v) => v.get(self.idx).copied().map(Value::ScalarBool).ok_or(
                SoAKitError::IndexOutOfBounds {
                    index: self.idx,
                    max: v.len(),
                },
            ),
            Value::VectorString(v) => v.get(self.idx).cloned().map(Value::ScalarString).ok_or(
                SoAKitError::IndexOutOfBounds {
                    index: self.idx,
                    max: v.len(),
                },
            ),
            _ => Err(SoAKitError::InvalidArgument(
                "Field value is not a vector".to_string(),
            )),
        }
    }

    /// Get the index this proxy represents.
    ///
    /// # Returns
    ///
    /// The index of the element this proxy represents as a `usize`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Bulk;
    ///
    /// let bulk = Bulk::new(5).unwrap();
    /// let proxy = bulk.at(2).unwrap();
    /// assert_eq!(proxy.index(), 2);
    /// ```
    pub fn index(&self) -> usize {
        self.idx
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
    /// use soakit::Bulk;
    ///
    /// let bulk = Bulk::new(3).unwrap();
    /// let proxy = bulk.at(1).unwrap();
    /// assert_eq!(proxy.bulk().count(), 3);
    /// ```
    pub fn bulk(&self) -> &Bulk {
        &self.bulk
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_proxy_creation() {
        let bulk = Rc::new(Bulk::new(5).unwrap());
        let proxy = Proxy::new(bulk.clone(), 2).unwrap();
        assert_eq!(proxy.index(), 2);
    }

    #[test]
    fn test_proxy_out_of_bounds() {
        let bulk = Rc::new(Bulk::new(5).unwrap());
        let result = Proxy::new(bulk, 10);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SoAKitError::IndexOutOfBounds { .. }
        ));
    }

    #[test]
    fn test_proxy_get_field() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("age".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Rc::new(Bulk::new(3).unwrap());
        let values = vec![
            Value::ScalarInt(10),
            Value::ScalarInt(20),
            Value::ScalarInt(30),
        ];
        let bulk = Rc::new(bulk.set(&registry, "age", values).unwrap());

        let proxy = Proxy::new(bulk.clone(), 1).unwrap();
        let value = proxy.get_field(&registry, "age").unwrap();
        assert_eq!(value, Value::ScalarInt(20));
    }

    #[test]
    fn test_proxy_get_field_all_types() {
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

        let bulk = Rc::new(Bulk::new(2).unwrap());
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "age",
                vec![Value::ScalarInt(25), Value::ScalarInt(30)],
            )
            .unwrap(),
        );
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "height",
                vec![Value::ScalarFloat(1.75), Value::ScalarFloat(1.80)],
            )
            .unwrap(),
        );
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "active",
                vec![Value::ScalarBool(true), Value::ScalarBool(false)],
            )
            .unwrap(),
        );
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "name",
                vec![
                    Value::ScalarString("Alice".to_string()),
                    Value::ScalarString("Bob".to_string()),
                ],
            )
            .unwrap(),
        );

        let proxy = Proxy::new(bulk.clone(), 0).unwrap();
        assert_eq!(
            proxy.get_field(&registry, "age").unwrap(),
            Value::ScalarInt(25)
        );
        assert_eq!(
            proxy.get_field(&registry, "height").unwrap(),
            Value::ScalarFloat(1.75)
        );
        assert_eq!(
            proxy.get_field(&registry, "active").unwrap(),
            Value::ScalarBool(true)
        );
        assert_eq!(
            proxy.get_field(&registry, "name").unwrap(),
            Value::ScalarString("Alice".to_string())
        );

        let proxy = Proxy::new(bulk.clone(), 1).unwrap();
        assert_eq!(
            proxy.get_field(&registry, "age").unwrap(),
            Value::ScalarInt(30)
        );
        assert_eq!(
            proxy.get_field(&registry, "height").unwrap(),
            Value::ScalarFloat(1.80)
        );
        assert_eq!(
            proxy.get_field(&registry, "active").unwrap(),
            Value::ScalarBool(false)
        );
        assert_eq!(
            proxy.get_field(&registry, "name").unwrap(),
            Value::ScalarString("Bob".to_string())
        );
    }

    #[test]
    fn test_proxy_boundary_conditions() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("value".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Rc::new(Bulk::new(5).unwrap());
        let values = vec![
            Value::ScalarInt(0),
            Value::ScalarInt(1),
            Value::ScalarInt(2),
            Value::ScalarInt(3),
            Value::ScalarInt(4),
        ];
        let bulk = Rc::new(bulk.set(&registry, "value", values).unwrap());

        // First element
        let proxy = Proxy::new(bulk.clone(), 0).unwrap();
        assert_eq!(
            proxy.get_field(&registry, "value").unwrap(),
            Value::ScalarInt(0)
        );

        // Last element
        let proxy = Proxy::new(bulk.clone(), 4).unwrap();
        assert_eq!(
            proxy.get_field(&registry, "value").unwrap(),
            Value::ScalarInt(4)
        );
    }

    #[test]
    fn test_proxy_single_element() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarInt(_)));
        registry
            .register("value".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Rc::new(Bulk::new(1).unwrap());
        let bulk = Rc::new(
            bulk.set(&registry, "value", vec![Value::ScalarInt(42)])
                .unwrap(),
        );

        let proxy = Proxy::new(bulk.clone(), 0).unwrap();
        assert_eq!(
            proxy.get_field(&registry, "value").unwrap(),
            Value::ScalarInt(42)
        );
    }

    #[test]
    fn test_proxy_get_field_nonexistent() {
        let registry = Registry::new();
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let proxy = Proxy::new(bulk.clone(), 0).unwrap();

        let result = proxy.get_field(&registry, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SoAKitError::FieldNotFound(_)));
    }

    #[test]
    fn test_proxy_index_at_boundary() {
        let bulk = Rc::new(Bulk::new(5).unwrap());

        // Index at max (should fail)
        let result = Proxy::new(bulk.clone(), 5);
        assert!(result.is_err());

        // Index just below max (should succeed)
        let proxy = Proxy::new(bulk.clone(), 4).unwrap();
        assert_eq!(proxy.index(), 4);
    }

    #[test]
    fn test_proxy_bulk_reference() {
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let proxy = Proxy::new(bulk.clone(), 1).unwrap();

        let bulk_ref = proxy.bulk();
        assert_eq!(bulk_ref.count(), 3);
    }

    #[test]
    fn test_proxy_with_derived_field() {
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
                Err(SoAKitError::InvalidArgument(
                    "Invalid arguments".to_string(),
                ))
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

        let bulk = Rc::new(Bulk::new(3).unwrap());
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "a",
                vec![
                    Value::ScalarInt(10),
                    Value::ScalarInt(20),
                    Value::ScalarInt(30),
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
                ],
            )
            .unwrap(),
        );

        let proxy = Proxy::new(bulk.clone(), 1).unwrap();
        let sum_value = proxy.get_field(&registry, "sum").unwrap();
        assert_eq!(sum_value, Value::ScalarInt(35)); // 20 + 15
    }

    #[test]
    fn test_proxy_zero_index() {
        let bulk = Rc::new(Bulk::new(3).unwrap());
        let proxy = Proxy::new(bulk.clone(), 0).unwrap();
        assert_eq!(proxy.index(), 0);
    }

    #[test]
    fn test_proxy_with_empty_strings() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarString(_)));
        registry
            .register("name".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Rc::new(Bulk::new(2).unwrap());
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "name",
                vec![
                    Value::ScalarString(String::new()),
                    Value::ScalarString("test".to_string()),
                ],
            )
            .unwrap(),
        );

        let proxy = Proxy::new(bulk.clone(), 0).unwrap();
        assert_eq!(
            proxy.get_field(&registry, "name").unwrap(),
            Value::ScalarString(String::new())
        );
    }

    #[test]
    fn test_proxy_with_special_float_values() {
        let mut registry = Registry::new();
        let validator = Box::new(|v: &Value| matches!(v, Value::ScalarFloat(_)));
        registry
            .register("value".to_string(), validator, false, vec![], None)
            .unwrap();

        let bulk = Rc::new(Bulk::new(3).unwrap());
        let bulk = Rc::new(
            bulk.set(
                &registry,
                "value",
                vec![
                    Value::ScalarFloat(f64::NAN),
                    Value::ScalarFloat(f64::INFINITY),
                    Value::ScalarFloat(3.14),
                ],
            )
            .unwrap(),
        );

        let proxy = Proxy::new(bulk.clone(), 0).unwrap();
        let nan_value = proxy.get_field(&registry, "value").unwrap();
        if let Value::ScalarFloat(f) = nan_value {
            assert!(f.is_nan());
        } else {
            panic!("Expected ScalarFloat");
        }

        let proxy = Proxy::new(bulk.clone(), 1).unwrap();
        let inf_value = proxy.get_field(&registry, "value").unwrap();
        if let Value::ScalarFloat(f) = inf_value {
            assert!(f.is_infinite());
        } else {
            panic!("Expected ScalarFloat");
        }
    }
}
