//! The Python boundary for the translated functions.

use std::collections::{HashMap, HashSet};

use pyo3::exceptions::{
    PyIndexError, PyKeyError, PyOverflowError, PyValueError, PyZeroDivisionError,
};
use pyo3::prelude::*;

use crate::compat::RuntimeError;
use crate::generated;

/// Map a compiled function's failure onto the exception Python raises for the same condition.
fn __compylr_to_py_err(error: RuntimeError) -> PyErr {
    match error {
        RuntimeError::DivisionByZero => PyZeroDivisionError::new_err("division by zero"),
        RuntimeError::Overflow => {
            PyOverflowError::new_err("integer arithmetic overflowed a 64-bit signed integer")
        }
        RuntimeError::IndexOutOfRange => PyIndexError::new_err("index out of range"),
        RuntimeError::ZeroStep => PyValueError::new_err("range() arg 3 must not be zero"),
        RuntimeError::MissingKey(key) => PyKeyError::new_err(key),
    }
}

/// An nth-prime that remembers what it has already computed.
#[pyclass(name = "PrimeCache")]
pub struct __compylr_class_0 {
    inner: generated::PrimeCache,
}

#[pymethods]
impl __compylr_class_0 {
    #[new]
    fn __compylr_init() -> PyResult<Self> {
        Ok(Self { inner: generated::PrimeCache::__compylr_new().map_err(__compylr_to_py_err)? })
    }

    /// How many requests were answered from the cache.
    #[pyo3(name = "hit_count")]
    fn __compylr_method_0_0(&self) -> PyResult<i64> {
        self.inner.hit_count().map_err(__compylr_to_py_err)
    }

    #[pyo3(name = "is_prime")]
    fn __compylr_method_0_1(&self, n: i64) -> PyResult<bool> {
        self.inner.is_prime(n).map_err(__compylr_to_py_err)
    }

    /// How many answers are cached.
    #[pyo3(name = "known_count")]
    fn __compylr_method_0_2(&self) -> PyResult<i64> {
        self.inner.known_count().map_err(__compylr_to_py_err)
    }

    /// The `n`th prime, one-indexed. Returns 0 for `n` below one.
    #[pyo3(name = "nth")]
    fn __compylr_method_0_3(&mut self, n: i64) -> PyResult<i64> {
        self.inner.nth(n).map_err(__compylr_to_py_err)
    }
}

#[pyfunction]
#[pyo3(name = "iterative_not_divisible")]
fn __compylr_export_0(divisible: bool) -> PyResult<bool> {
    generated::iterative_not_divisible(divisible).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "iterative_nth_prime")]
fn __compylr_export_1(n: i64) -> PyResult<i64> {
    generated::iterative_nth_prime(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "iterative_primes_up_to_count")]
fn __compylr_export_2(n: i64) -> PyResult<Vec<i64>> {
    generated::iterative_primes_up_to_count(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "recursive_is_prime")]
fn __compylr_export_3(n: i64) -> PyResult<bool> {
    generated::recursive_is_prime(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "recursive_next_prime")]
fn __compylr_export_4(after: i64) -> PyResult<i64> {
    generated::recursive_next_prime(after).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "recursive_nth_prime")]
fn __compylr_export_5(n: i64) -> PyResult<i64> {
    generated::recursive_nth_prime(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "recursive_nth_prime_from")]
fn __compylr_export_6(remaining: i64, current: i64) -> PyResult<i64> {
    generated::recursive_nth_prime_from(remaining, current).map_err(__compylr_to_py_err)
}

/// Register every compiled function on the module.
///
/// Kept here rather than in the crate root so that `wrap_pyfunction!` resolves the
/// wrappers locally, and so the root stays the same size for any number of functions.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<__compylr_class_0>()?;
    m.add_function(wrap_pyfunction!(__compylr_export_0, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_1, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_2, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_3, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_4, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_5, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_6, m)?)?;
    Ok(())
}
