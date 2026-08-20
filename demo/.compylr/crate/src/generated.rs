//! Translated by compylr.

use std::collections::{HashMap, HashSet};

use crate::compat::{
    IndexOrigin, PyAdd, PyContains, PyIterate, PyLen, PyNum, PySetItem, RuntimeError, TextUnits,
    div_exact, py_subscript,
};

/// An nth-prime that remembers what it has already computed.
#[derive(Clone)]
pub struct PrimeCache {
    pub known: HashMap<i64, i64>,
    pub hits: i64,
}

impl PrimeCache {
    pub fn __compylr_new() -> Result<Self, RuntimeError> {
        let known: HashMap<i64, i64> = HashMap::from([]);
        let hits: i64 = 0i64;
        Ok(Self { known, hits })
    }

    /// How many requests were answered from the cache.
    pub fn hit_count(&self) -> Result<i64, RuntimeError> {
        Ok((self).hits.clone())
    }

    pub fn is_prime(&self, n: i64) -> Result<bool, RuntimeError> {
        if ((&(n)) < (&(2i64))) {
            return Ok(false);
        }
        let mut d: i64 = 2i64;
        while ((&(PyNum::py_mul(&(d), &(d))?)) <= (&(n))) {
            if ((&(PyNum::rem_floor(&(n), &(d))?)) == (&(0i64))) {
                return Ok(false);
            }
            d = PyAdd::py_add(&(d), &(1i64))?;
        }
        Ok(true)
    }

    /// How many answers are cached.
    pub fn known_count(&self) -> Result<i64, RuntimeError> {
        Ok(PyLen::py_len(
            &((self).known.clone()),
            TextUnits::CodePoints,
        ))
    }

    /// The `n`th prime, one-indexed. Returns 0 for `n` below one.
    pub fn nth(&mut self, n: i64) -> Result<i64, RuntimeError> {
        if ((&(n)) < (&(1i64))) {
            return Ok(0i64);
        }
        if PyContains::py_contains(&((self).known.clone()), &(n)) {
            (self).hits = PyAdd::py_add(&((self).hits.clone()), &(1i64))?;
            return Ok(py_subscript(
                &((self).known.clone()),
                &(n),
                IndexOrigin::FromEitherEnd,
            )?);
        }
        let mut found: i64 = 0i64;
        let mut candidate: i64 = 1i64;
        while ((&(found)) < (&(n))) {
            candidate = PyAdd::py_add(&(candidate), &(1i64))?;
            if (self).is_prime(candidate)? {
                found = PyAdd::py_add(&(found), &(1i64))?;
            }
        }
        {
            let __compylr_value = candidate.clone();
            let __compylr_index = n.clone();
            PySetItem::py_set(&mut ((self).known), &__compylr_index, __compylr_value)?;
        }
        Ok(candidate)
    }
}

/// Negation, written out because the subset has no `not` operator.
///
///     `return not divisible` is what anyone would write, and is rejected. Recorded in the README as a
///     gap this demo found rather than worked around silently.
///     
pub fn iterative_not_divisible(divisible: bool) -> Result<bool, RuntimeError> {
    if divisible {
        return Ok(false);
    }
    Ok(true)
}

/// The `n`th prime, one-indexed. Returns 0 for `n` below one.
pub fn iterative_nth_prime(n: i64) -> Result<i64, RuntimeError> {
    if ((&(n)) < (&(1i64))) {
        return Ok(0i64);
    }
    let found: Vec<i64> = iterative_primes_up_to_count(n)?;
    Ok(py_subscript(
        &(found),
        &(PyNum::py_sub(&(n), &(1i64))?),
        IndexOrigin::FromEitherEnd,
    )?)
}

/// The first `n` primes, in order.
pub fn iterative_primes_up_to_count(n: i64) -> Result<Vec<i64>, RuntimeError> {
    let mut found: Vec<i64> = vec![];
    let mut candidate: i64 = 2i64;
    while ((&(PyLen::py_len(&(found), TextUnits::CodePoints))) < (&(n))) {
        let mut divisible: bool = false;
        {
            let __compylr_iter = &found;
            for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                let p: i64 = __compylr_item;
                if ((&(PyNum::py_mul(&(p), &(p))?)) > (&(candidate))) {
                    break;
                }
                if ((&(PyNum::rem_floor(&(candidate), &(p))?)) == (&(0i64))) {
                    divisible = true;
                    break;
                }
            }
        }
        if iterative_not_divisible(divisible)? {
            {
                let __compylr_value = candidate.clone();
                (found).push(__compylr_value);
            }
        }
        candidate = PyAdd::py_add(&(candidate), &(1i64))?;
    }
    Ok(found.clone())
}

/// Whether `n` is prime, by trial division.
pub fn recursive_is_prime(n: i64) -> Result<bool, RuntimeError> {
    if ((&(n)) < (&(2i64))) {
        return Ok(false);
    }
    let mut d: i64 = 2i64;
    while ((&(PyNum::py_mul(&(d), &(d))?)) <= (&(n))) {
        if ((&(PyNum::rem_floor(&(n), &(d))?)) == (&(0i64))) {
            return Ok(false);
        }
        d = PyAdd::py_add(&(d), &(1i64))?;
    }
    Ok(true)
}

/// The smallest prime strictly greater than `after`.
pub fn recursive_next_prime(after: i64) -> Result<i64, RuntimeError> {
    let mut candidate: i64 = PyAdd::py_add(&(after), &(1i64))?;
    let mut found: i64 = 0i64;
    while ((&(found)) == (&(0i64))) {
        if recursive_is_prime(candidate)? {
            found = candidate;
        }
        candidate = PyAdd::py_add(&(candidate), &(1i64))?;
    }
    Ok(found)
}

/// The `n`th prime, one-indexed. Returns 0 for `n` below one.
pub fn recursive_nth_prime(n: i64) -> Result<i64, RuntimeError> {
    if ((&(n)) < (&(1i64))) {
        return Ok(0i64);
    }
    Ok(recursive_nth_prime_from(n, 1i64)?)
}

/// The `remaining`th prime after `current`, recursing once per prime rather than per integer.
pub fn recursive_nth_prime_from(remaining: i64, current: i64) -> Result<i64, RuntimeError> {
    if ((&(remaining)) < (&(1i64))) {
        return Ok(current);
    }
    Ok(recursive_nth_prime_from(
        PyNum::py_sub(&(remaining), &(1i64))?,
        recursive_next_prime(current)?,
    )?)
}
