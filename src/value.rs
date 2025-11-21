/// Value types for SoAKit
///
/// This module defines the [`Value`] enum which represents all possible data types
/// that can be stored in a SoAKit [`Bulk`] structure. Values can be scalars (rank 0),
/// vectors (rank 1), or matrices (rank 2+).
use crate::error::{Result, SoAKitError};
use std::fmt;

/// Represents a value in the SoA structure.
///
/// Values can be scalars (single values), vectors (1D arrays), or matrices (2D+ arrays).
/// Each variant supports different primitive types: integers, floats, booleans, and strings.
///
/// # Type Hierarchy
///
/// - **Scalars** (rank 0): Single values
///   - `ScalarInt(i64)`: 64-bit signed integer
///   - `ScalarFloat(f64)`: 64-bit floating-point number
///   - `ScalarBool(bool)`: Boolean value
///   - `ScalarString(String)`: String value
///
/// - **Vectors** (rank 1): 1D arrays
///   - `VectorInt(Vec<i64>)`: Vector of integers
///   - `VectorFloat(Vec<f64>)`: Vector of floats
///   - `VectorBool(Vec<bool>)`: Vector of booleans
///   - `VectorString(Vec<String>)`: Vector of strings
///
/// - **Matrices** (rank 2+): Nested structures
///   - `Matrix(Vec<Value>)`: Matrix represented as a vector of Value elements
///
/// # Examples
///
/// Creating scalar values:
///
/// ```rust
/// use soakit::Value;
///
/// let age = Value::ScalarInt(25);
/// let height = Value::ScalarFloat(1.75);
/// let active = Value::ScalarBool(true);
/// let name = Value::ScalarString("Alice".to_string());
/// ```
///
/// Creating vector values:
///
/// ```rust
/// use soakit::Value;
///
/// let ages = Value::VectorInt(vec![25, 30, 35]);
/// let heights = Value::VectorFloat(vec![1.75, 1.80, 1.65]);
/// let flags = Value::VectorBool(vec![true, false, true]);
/// let names = Value::VectorString(vec!["Alice".to_string(), "Bob".to_string()]);
/// ```
///
/// Creating matrix values:
///
/// ```rust
/// use soakit::Value;
///
/// let matrix = Value::Matrix(vec![
///     Value::VectorInt(vec![1, 2, 3]),
///     Value::VectorInt(vec![4, 5, 6]),
/// ]);
/// ```
#[derive(Clone, PartialEq)]
pub enum Value {
    /// Scalar integer value (64-bit signed integer)
    ScalarInt(i64),
    /// Scalar float value (64-bit floating-point number)
    ScalarFloat(f64),
    /// Scalar boolean value
    ScalarBool(bool),
    /// Scalar string value
    ScalarString(String),
    /// Vector of integers
    VectorInt(Vec<i64>),
    /// Vector of floats
    VectorFloat(Vec<f64>),
    /// Vector of booleans
    VectorBool(Vec<bool>),
    /// Vector of strings
    VectorString(Vec<String>),
    /// Matrix (nested vectors) - represented as Vec<Value>
    ///
    /// Each element in the vector represents a row, and each row is itself a Value
    /// (typically a Vector variant).
    Matrix(Vec<Value>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::ScalarInt(v) => write!(f, "ScalarInt({})", v),
            Value::ScalarFloat(v) => write!(f, "ScalarFloat({})", v),
            Value::ScalarBool(v) => write!(f, "ScalarBool({})", v),
            Value::ScalarString(v) => write!(f, "ScalarString({:?})", v),
            Value::VectorInt(v) => write!(f, "VectorInt({:?})", v),
            Value::VectorFloat(v) => write!(f, "VectorFloat({:?})", v),
            Value::VectorBool(v) => write!(f, "VectorBool({:?})", v),
            Value::VectorString(v) => write!(f, "VectorString({:?})", v),
            Value::Matrix(v) => write!(f, "Matrix({:?})", v),
        }
    }
}

impl Value {
    /// Check if the value is a scalar (rank 0).
    ///
    /// Scalars are single values, not collections.
    ///
    /// # Returns
    ///
    /// `true` if the value is a scalar variant (`ScalarInt`, `ScalarFloat`, `ScalarBool`, or `ScalarString`),
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Value;
    ///
    /// assert!(Value::ScalarInt(42).is_scalar());
    /// assert!(Value::ScalarFloat(3.14).is_scalar());
    /// assert!(!Value::VectorInt(vec![1, 2, 3]).is_scalar());
    /// ```
    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Value::ScalarInt(_)
                | Value::ScalarFloat(_)
                | Value::ScalarBool(_)
                | Value::ScalarString(_)
        )
    }

    /// Check if the value is a vector (rank 1).
    ///
    /// Vectors are 1D arrays of primitive values.
    ///
    /// # Returns
    ///
    /// `true` if the value is a vector variant (`VectorInt`, `VectorFloat`, `VectorBool`, or `VectorString`),
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Value;
    ///
    /// assert!(Value::VectorInt(vec![1, 2, 3]).is_vector());
    /// assert!(!Value::ScalarInt(42).is_vector());
    /// assert!(!Value::Matrix(vec![]).is_vector());
    /// ```
    pub fn is_vector(&self) -> bool {
        matches!(
            self,
            Value::VectorInt(_)
                | Value::VectorFloat(_)
                | Value::VectorBool(_)
                | Value::VectorString(_)
        )
    }

    /// Check if the value is a matrix (rank 2+).
    ///
    /// Matrices are nested structures represented as vectors of Value elements.
    ///
    /// # Returns
    ///
    /// `true` if the value is a `Matrix` variant, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Value;
    ///
    /// let matrix = Value::Matrix(vec![Value::VectorInt(vec![1, 2])]);
    /// assert!(matrix.is_matrix());
    /// assert!(!Value::VectorInt(vec![1, 2]).is_matrix());
    /// ```
    pub fn is_matrix(&self) -> bool {
        matches!(self, Value::Matrix(_))
    }

    /// Get the rank (number of dimensions) of the value.
    ///
    /// The rank indicates the dimensionality of the value:
    /// - `0` for scalars
    /// - `1` for vectors
    /// - `2` for matrices (and higher-dimensional structures)
    ///
    /// # Returns
    ///
    /// The rank of the value as a `usize`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Value;
    ///
    /// assert_eq!(Value::ScalarInt(42).rank(), 0);
    /// assert_eq!(Value::VectorInt(vec![1, 2, 3]).rank(), 1);
    /// assert_eq!(Value::Matrix(vec![Value::VectorInt(vec![1, 2])]).rank(), 2);
    /// ```
    pub fn rank(&self) -> usize {
        match self {
            Value::ScalarInt(_)
            | Value::ScalarFloat(_)
            | Value::ScalarBool(_)
            | Value::ScalarString(_) => 0,
            Value::VectorInt(_)
            | Value::VectorFloat(_)
            | Value::VectorBool(_)
            | Value::VectorString(_) => 1,
            Value::Matrix(_) => 2,
        }
    }

    /// Get the length of the value.
    ///
    /// For scalars, this always returns `1` (a scalar is considered to have length 1).
    /// For vectors, this returns the number of elements in the vector.
    /// For matrices, this returns the number of rows.
    ///
    /// # Returns
    ///
    /// The length of the value as a `usize`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Value;
    ///
    /// assert_eq!(Value::ScalarInt(42).len(), 1);
    /// assert_eq!(Value::VectorInt(vec![1, 2, 3]).len(), 3);
    /// assert_eq!(Value::VectorInt(vec![]).len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        match self {
            Value::ScalarInt(_)
            | Value::ScalarFloat(_)
            | Value::ScalarBool(_)
            | Value::ScalarString(_) => 1,
            Value::VectorInt(v) => v.len(),
            Value::VectorFloat(v) => v.len(),
            Value::VectorBool(v) => v.len(),
            Value::VectorString(v) => v.len(),
            Value::Matrix(v) => v.len(),
        }
    }

    /// Check if the value is empty.
    ///
    /// A value is considered empty if its length is 0. Note that scalars
    /// are never empty (they always have length 1).
    ///
    /// # Returns
    ///
    /// `true` if the value has length 0, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Value;
    ///
    /// assert!(!Value::ScalarInt(0).is_empty());
    /// assert!(Value::VectorInt(vec![]).is_empty());
    /// assert!(!Value::VectorInt(vec![1, 2]).is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the shape (dimensions) of the value.
    ///
    /// The shape is a vector representing the size of each dimension.
    /// - Scalars return an empty vector `[]`
    /// - Vectors return `[length]`
    /// - Matrices return `[rows, columns]`
    ///
    /// # Returns
    ///
    /// A vector of `usize` values representing the dimensions of the value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Value;
    ///
    /// assert_eq!(Value::ScalarInt(42).shape(), vec![]);
    /// assert_eq!(Value::VectorInt(vec![1, 2, 3]).shape(), vec![3]);
    /// let matrix = Value::Matrix(vec![
    ///     Value::VectorInt(vec![1, 2]),
    ///     Value::VectorInt(vec![3, 4]),
    /// ]);
    /// assert_eq!(matrix.shape(), vec![2, 2]);
    /// ```
    pub fn shape(&self) -> Vec<usize> {
        match self {
            Value::ScalarInt(_)
            | Value::ScalarFloat(_)
            | Value::ScalarBool(_)
            | Value::ScalarString(_) => vec![],
            Value::VectorInt(v) => vec![v.len()],
            Value::VectorFloat(v) => vec![v.len()],
            Value::VectorBool(v) => vec![v.len()],
            Value::VectorString(v) => vec![v.len()],
            Value::Matrix(m) => {
                if m.is_empty() {
                    vec![0]
                } else {
                    let first_row_len = m
                        .first()
                        .map(|row| row.len())
                        .unwrap_or(0);
                    vec![m.len(), first_row_len]
                }
            }
        }
    }

    /// Extract a single element from a vector by index.
    ///
    /// This method extracts the element at the given index from a vector value
    /// and returns it as a scalar value of the same type.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the element to extract (0-based)
    ///
    /// # Returns
    ///
    /// Returns `Ok(Value)` containing the scalar value at the given index, or an error if:
    /// - The value is not a vector
    /// - The index is out of bounds
    ///
    /// # Errors
    ///
    /// - [`SoAKitError::IndexOutOfBounds`] if the index is out of bounds
    /// - [`SoAKitError::InvalidArgument`] if the value is not a vector
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soakit::Value;
    ///
    /// let vec = Value::VectorInt(vec![10, 20, 30]);
    /// assert_eq!(vec.get_element(0).unwrap(), Value::ScalarInt(10));
    /// assert_eq!(vec.get_element(1).unwrap(), Value::ScalarInt(20));
    ///
    /// // Out of bounds
    /// assert!(vec.get_element(10).is_err());
    ///
    /// // Not a vector
    /// assert!(Value::ScalarInt(42).get_element(0).is_err());
    /// ```
    pub fn get_element(&self, idx: usize) -> Result<Value> {
        match self {
            Value::VectorInt(v) => {
                v.get(idx)
                    .copied()
                    .map(Value::ScalarInt)
                    .ok_or_else(|| SoAKitError::IndexOutOfBounds {
                        index: idx,
                        max: v.len(),
                    })
            }
            Value::VectorFloat(v) => {
                v.get(idx)
                    .copied()
                    .map(Value::ScalarFloat)
                    .ok_or_else(|| SoAKitError::IndexOutOfBounds {
                        index: idx,
                        max: v.len(),
                    })
            }
            Value::VectorBool(v) => {
                v.get(idx)
                    .copied()
                    .map(Value::ScalarBool)
                    .ok_or_else(|| SoAKitError::IndexOutOfBounds {
                        index: idx,
                        max: v.len(),
                    })
            }
            Value::VectorString(v) => {
                v.get(idx)
                    .cloned()
                    .map(Value::ScalarString)
                    .ok_or_else(|| SoAKitError::IndexOutOfBounds {
                        index: idx,
                        max: v.len(),
                    })
            }
            _ => Err(SoAKitError::InvalidArgument(
                "get_element only works on vectors".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_types() {
        let v1 = Value::ScalarInt(42);
        let v2 = Value::ScalarFloat(3.14);
        let v3 = Value::ScalarBool(true);
        let v4 = Value::ScalarString("hello".to_string());

        assert!(v1.is_scalar());
        assert!(v2.is_scalar());
        assert!(v3.is_scalar());
        assert!(v4.is_scalar());
        assert!(!v1.is_vector());
        assert!(!v1.is_matrix());
        assert_eq!(v1.rank(), 0);
        assert_eq!(v1.len(), 1);
    }

    #[test]
    fn test_vector_types() {
        let v1 = Value::VectorInt(vec![1, 2, 3]);
        let v2 = Value::VectorFloat(vec![1.0, 2.0, 3.0]);
        let v3 = Value::VectorBool(vec![true, false]);
        let v4 = Value::VectorString(vec!["a".to_string(), "b".to_string()]);

        assert!(v1.is_vector());
        assert!(v2.is_vector());
        assert!(v3.is_vector());
        assert!(v4.is_vector());
        assert!(!v1.is_scalar());
        assert!(!v1.is_matrix());
        assert_eq!(v1.rank(), 1);
        assert_eq!(v1.len(), 3);
        assert_eq!(v3.len(), 2);
    }

    #[test]
    fn test_matrix_type() {
        let m = Value::Matrix(vec![
            Value::VectorInt(vec![1, 2]),
            Value::VectorInt(vec![3, 4]),
        ]);
        assert!(m.is_matrix());
        assert!(!m.is_scalar());
        assert!(!m.is_vector());
        assert_eq!(m.rank(), 2);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_get_element() {
        let v = Value::VectorInt(vec![10, 20, 30]);
        assert_eq!(v.get_element(0).unwrap(), Value::ScalarInt(10));
        assert_eq!(v.get_element(1).unwrap(), Value::ScalarInt(20));
        assert!(v.get_element(10).is_err());
    }

    #[test]
    fn test_scalar_edge_values() {
        // Test zero, negative, and extreme values
        let zero_int = Value::ScalarInt(0);
        let neg_int = Value::ScalarInt(-42);
        let max_int = Value::ScalarInt(i64::MAX);
        let min_int = Value::ScalarInt(i64::MIN);

        assert!(zero_int.is_scalar());
        assert_eq!(zero_int.rank(), 0);
        assert_eq!(zero_int.len(), 1);
        assert!(!zero_int.is_empty());

        assert!(neg_int.is_scalar());
        assert!(max_int.is_scalar());
        assert!(min_int.is_scalar());

        // Test float edge values
        let zero_float = Value::ScalarFloat(0.0);
        let neg_float = Value::ScalarFloat(-3.14);
        let inf_float = Value::ScalarFloat(f64::INFINITY);
        let neg_inf_float = Value::ScalarFloat(f64::NEG_INFINITY);
        let nan_float = Value::ScalarFloat(f64::NAN);

        assert!(zero_float.is_scalar());
        assert!(neg_float.is_scalar());
        assert!(inf_float.is_scalar());
        assert!(neg_inf_float.is_scalar());
        assert!(nan_float.is_scalar());

        // Test empty string
        let empty_str = Value::ScalarString(String::new());
        assert!(empty_str.is_scalar());
        assert_eq!(empty_str.len(), 1);
        assert!(!empty_str.is_empty());
    }

    #[test]
    fn test_empty_vectors() {
        let empty_int = Value::VectorInt(vec![]);
        let empty_float = Value::VectorFloat(vec![]);
        let empty_bool = Value::VectorBool(vec![]);
        let empty_string = Value::VectorString(vec![]);

        assert!(empty_int.is_vector());
        assert!(empty_int.is_empty());
        assert_eq!(empty_int.len(), 0);
        assert_eq!(empty_int.rank(), 1);
        assert_eq!(empty_int.shape(), vec![0]);

        assert!(empty_float.is_empty());
        assert!(empty_bool.is_empty());
        assert!(empty_string.is_empty());
    }

    #[test]
    fn test_single_element_vectors() {
        let single_int = Value::VectorInt(vec![42]);
        let single_float = Value::VectorFloat(vec![3.14]);
        let single_bool = Value::VectorBool(vec![true]);
        let single_string = Value::VectorString(vec!["hello".to_string()]);

        assert!(single_int.is_vector());
        assert_eq!(single_int.len(), 1);
        assert!(!single_int.is_empty());
        assert_eq!(single_int.shape(), vec![1]);
        assert_eq!(single_int.get_element(0).unwrap(), Value::ScalarInt(42));

        assert_eq!(single_float.len(), 1);
        assert_eq!(single_bool.len(), 1);
        assert_eq!(single_string.len(), 1);
    }

    #[test]
    fn test_large_vectors() {
        let large_int = Value::VectorInt((0..1000).collect());
        assert_eq!(large_int.len(), 1000);
        assert_eq!(large_int.get_element(0).unwrap(), Value::ScalarInt(0));
        assert_eq!(large_int.get_element(999).unwrap(), Value::ScalarInt(999));
        assert!(large_int.get_element(1000).is_err());

        let large_float = Value::VectorFloat((0..100).map(|i| i as f64).collect());
        assert_eq!(large_float.len(), 100);
    }

    #[test]
    fn test_vector_float_nan_handling() {
        let vec_with_nan = Value::VectorFloat(vec![1.0, f64::NAN, 3.0]);
        assert_eq!(vec_with_nan.len(), 3);
        assert_eq!(vec_with_nan.get_element(0).unwrap(), Value::ScalarFloat(1.0));
        
        // NaN comparison - get_element should work
        let nan_elem = vec_with_nan.get_element(1).unwrap();
        if let Value::ScalarFloat(f) = nan_elem {
            assert!(f.is_nan());
        } else {
            panic!("Expected ScalarFloat");
        }

        assert_eq!(vec_with_nan.get_element(2).unwrap(), Value::ScalarFloat(3.0));
    }

    #[test]
    fn test_get_element_all_vector_types() {
        // Test VectorInt
        let v_int = Value::VectorInt(vec![1, 2, 3]);
        assert_eq!(v_int.get_element(0).unwrap(), Value::ScalarInt(1));
        assert_eq!(v_int.get_element(2).unwrap(), Value::ScalarInt(3));
        assert!(v_int.get_element(3).is_err());

        // Test VectorFloat
        let v_float = Value::VectorFloat(vec![1.1, 2.2, 3.3]);
        assert_eq!(v_float.get_element(0).unwrap(), Value::ScalarFloat(1.1));
        assert_eq!(v_float.get_element(2).unwrap(), Value::ScalarFloat(3.3));

        // Test VectorBool
        let v_bool = Value::VectorBool(vec![true, false, true]);
        assert_eq!(v_bool.get_element(0).unwrap(), Value::ScalarBool(true));
        assert_eq!(v_bool.get_element(1).unwrap(), Value::ScalarBool(false));

        // Test VectorString
        let v_string = Value::VectorString(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(v_string.get_element(0).unwrap(), Value::ScalarString("a".to_string()));
        assert_eq!(v_string.get_element(2).unwrap(), Value::ScalarString("c".to_string()));
    }

    #[test]
    fn test_get_element_non_vector() {
        // Should fail for scalars
        let scalar = Value::ScalarInt(42);
        assert!(scalar.get_element(0).is_err());
        assert!(matches!(
            scalar.get_element(0).unwrap_err(),
            SoAKitError::InvalidArgument(_)
        ));

        // Should fail for matrices
        let matrix = Value::Matrix(vec![Value::VectorInt(vec![1, 2])]);
        assert!(matrix.get_element(0).is_err());
    }

    #[test]
    fn test_shape_calculations() {
        // Scalar shapes
        assert_eq!(Value::ScalarInt(42).shape(), vec![]);
        assert_eq!(Value::ScalarFloat(3.14).shape(), vec![]);
        assert_eq!(Value::ScalarBool(true).shape(), vec![]);
        assert_eq!(Value::ScalarString("test".to_string()).shape(), vec![]);

        // Vector shapes
        assert_eq!(Value::VectorInt(vec![1, 2, 3]).shape(), vec![3]);
        assert_eq!(Value::VectorInt(vec![]).shape(), vec![0]);
        assert_eq!(Value::VectorFloat(vec![1.0, 2.0]).shape(), vec![2]);
        assert_eq!(Value::VectorString(vec!["a".to_string()]).shape(), vec![1]);

        // Matrix shapes
        let matrix = Value::Matrix(vec![
            Value::VectorInt(vec![1, 2, 3]),
            Value::VectorInt(vec![4, 5, 6]),
        ]);
        assert_eq!(matrix.shape(), vec![2, 3]);

        // Empty matrix
        let empty_matrix = Value::Matrix(vec![]);
        assert_eq!(empty_matrix.shape(), vec![0]);

        // Matrix with empty rows
        let matrix_empty_row = Value::Matrix(vec![
            Value::VectorInt(vec![]),
        ]);
        assert_eq!(matrix_empty_row.shape(), vec![1, 0]);
    }

    #[test]
    fn test_matrix_operations() {
        // Regular matrix
        let matrix = Value::Matrix(vec![
            Value::VectorInt(vec![1, 2]),
            Value::VectorInt(vec![3, 4]),
            Value::VectorInt(vec![5, 6]),
        ]);
        assert!(matrix.is_matrix());
        assert_eq!(matrix.rank(), 2);
        assert_eq!(matrix.len(), 3);
        assert_eq!(matrix.shape(), vec![3, 2]);
        assert!(!matrix.is_empty());

        // Empty matrix
        let empty_matrix = Value::Matrix(vec![]);
        assert!(empty_matrix.is_matrix());
        assert_eq!(empty_matrix.rank(), 2);
        assert_eq!(empty_matrix.len(), 0);
        assert!(empty_matrix.is_empty());

        // Single row matrix
        let single_row = Value::Matrix(vec![Value::VectorInt(vec![1, 2, 3])]);
        assert_eq!(single_row.len(), 1);
        assert_eq!(single_row.shape(), vec![1, 3]);
    }

    #[test]
    fn test_rank_calculations() {
        // All scalars have rank 0
        assert_eq!(Value::ScalarInt(0).rank(), 0);
        assert_eq!(Value::ScalarFloat(0.0).rank(), 0);
        assert_eq!(Value::ScalarBool(false).rank(), 0);
        assert_eq!(Value::ScalarString("".to_string()).rank(), 0);

        // All vectors have rank 1
        assert_eq!(Value::VectorInt(vec![]).rank(), 1);
        assert_eq!(Value::VectorFloat(vec![1.0]).rank(), 1);
        assert_eq!(Value::VectorBool(vec![true, false]).rank(), 1);
        assert_eq!(Value::VectorString(vec!["a".to_string()]).rank(), 1);

        // All matrices have rank 2
        assert_eq!(Value::Matrix(vec![]).rank(), 2);
        assert_eq!(Value::Matrix(vec![Value::VectorInt(vec![1])]).rank(), 2);
    }

    #[test]
    fn test_is_empty() {
        // Scalars are never empty
        assert!(!Value::ScalarInt(0).is_empty());
        assert!(!Value::ScalarString(String::new()).is_empty());

        // Empty vectors
        assert!(Value::VectorInt(vec![]).is_empty());
        assert!(Value::VectorFloat(vec![]).is_empty());
        assert!(Value::VectorBool(vec![]).is_empty());
        assert!(Value::VectorString(vec![]).is_empty());

        // Non-empty vectors
        assert!(!Value::VectorInt(vec![1]).is_empty());
        assert!(!Value::VectorFloat(vec![1.0, 2.0]).is_empty());

        // Empty matrix
        assert!(Value::Matrix(vec![]).is_empty());

        // Non-empty matrix
        assert!(!Value::Matrix(vec![Value::VectorInt(vec![1])]).is_empty());
    }

    #[test]
    fn test_string_edge_cases() {
        // Empty string scalar
        let empty = Value::ScalarString(String::new());
        assert_eq!(empty.len(), 1);
        assert!(!empty.is_empty());

        // String with special characters
        let special = Value::ScalarString("hello\nworld\t!".to_string());
        assert_eq!(special.len(), 1);

        // Unicode strings
        let unicode = Value::ScalarString("你好".to_string());
        assert_eq!(unicode.len(), 1);

        // Empty string vector
        let empty_vec = Value::VectorString(vec![]);
        assert!(empty_vec.is_empty());

        // Vector with empty strings
        let vec_empty_strs = Value::VectorString(vec![String::new(), "a".to_string()]);
        assert_eq!(vec_empty_strs.len(), 2);
        assert_eq!(vec_empty_strs.get_element(0).unwrap(), Value::ScalarString(String::new()));
    }

    #[test]
    fn test_boolean_edge_cases() {
        // All boolean combinations
        let all_true = Value::VectorBool(vec![true, true, true]);
        let all_false = Value::VectorBool(vec![false, false, false]);
        let mixed = Value::VectorBool(vec![true, false, true, false]);

        assert_eq!(all_true.len(), 3);
        assert_eq!(all_false.len(), 3);
        assert_eq!(mixed.len(), 4);

        assert_eq!(all_true.get_element(0).unwrap(), Value::ScalarBool(true));
        assert_eq!(all_false.get_element(0).unwrap(), Value::ScalarBool(false));
    }

    #[test]
    fn test_float_special_values() {
        let vec_special = Value::VectorFloat(vec![
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            0.0,
            -0.0,
        ]);

        assert_eq!(vec_special.len(), 5);
        let inf_elem = vec_special.get_element(0).unwrap();
        if let Value::ScalarFloat(f) = inf_elem {
            assert!(f.is_infinite() && f.is_sign_positive());
        } else {
            panic!("Expected ScalarFloat");
        }

        let neg_inf_elem = vec_special.get_element(1).unwrap();
        if let Value::ScalarFloat(f) = neg_inf_elem {
            assert!(f.is_infinite() && f.is_sign_negative());
        } else {
            panic!("Expected ScalarFloat");
        }
    }

    #[test]
    fn test_integer_edge_values() {
        let vec_extreme = Value::VectorInt(vec![
            i64::MIN,
            -1,
            0,
            1,
            i64::MAX,
        ]);

        assert_eq!(vec_extreme.len(), 5);
        assert_eq!(vec_extreme.get_element(0).unwrap(), Value::ScalarInt(i64::MIN));
        assert_eq!(vec_extreme.get_element(2).unwrap(), Value::ScalarInt(0));
        assert_eq!(vec_extreme.get_element(4).unwrap(), Value::ScalarInt(i64::MAX));
    }
}

