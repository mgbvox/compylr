//! The Python boundary for the translated functions.

// The hashed containers come from the runtime rather than from `std`. A wrapper signature has to
// name the same type the translated function does, and `std`'s aliases pin the default hasher.
use crate::compat::{FastMap, FastSet};

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

/// A stack of integers, over a list that never shrinks.
///
///     `append` is the only collection method in the subset — there is no `pop` — so the stack is a
///     list and a `height`. Pushing writes over a slot past the height when one is there and appends
///     otherwise, and popping just lowers the height. The list therefore grows to the deepest the
///     stack ever was and stays there, which is the allocation `pop` would have handed back and that
///     a stack is about to want again anyway.
///     
#[pyclass(name = "IntStack")]
pub struct __compylr_class_0 {
    inner: generated::IntStack,
}

#[pymethods]
impl __compylr_class_0 {
    #[new]
    fn __compylr_init() -> PyResult<Self> {
        Ok(Self {
            inner: generated::IntStack::__compylr_new().map_err(__compylr_to_py_err)?,
        })
    }

    /// How many values are on the stack.
    #[pyo3(name = "depth")]
    fn __compylr_method_0_0(&self) -> PyResult<i64> {
        self.inner.depth().map_err(__compylr_to_py_err)
    }

    /// The top value without removing it, or 0 when the stack is empty.
    #[pyo3(name = "peek")]
    fn __compylr_method_0_1(&self) -> PyResult<i64> {
        self.inner.peek().map_err(__compylr_to_py_err)
    }

    /// Take the top value off, or 0 when the stack is empty.
    #[pyo3(name = "pop")]
    fn __compylr_method_0_2(&mut self) -> PyResult<i64> {
        self.inner.pop().map_err(__compylr_to_py_err)
    }

    /// Put `value` on top.
    #[pyo3(name = "push")]
    fn __compylr_method_0_3(&mut self, value: i64) -> PyResult<()> {
        self.inner.push(value).map_err(__compylr_to_py_err)
    }
}

/// An nth-prime that remembers what it has already computed.
#[pyclass(name = "PrimeCache")]
pub struct __compylr_class_1 {
    inner: generated::PrimeCache,
}

#[pymethods]
impl __compylr_class_1 {
    #[new]
    fn __compylr_init() -> PyResult<Self> {
        Ok(Self {
            inner: generated::PrimeCache::__compylr_new().map_err(__compylr_to_py_err)?,
        })
    }

    /// How many requests were answered from the cache.
    #[pyo3(name = "hit_count")]
    fn __compylr_method_1_0(&self) -> PyResult<i64> {
        self.inner.hit_count().map_err(__compylr_to_py_err)
    }

    #[pyo3(name = "is_prime")]
    fn __compylr_method_1_1(&self, n: i64) -> PyResult<bool> {
        self.inner.is_prime(n).map_err(__compylr_to_py_err)
    }

    /// How many answers are cached.
    #[pyo3(name = "known_count")]
    fn __compylr_method_1_2(&self) -> PyResult<i64> {
        self.inner.known_count().map_err(__compylr_to_py_err)
    }

    /// The `n`th prime, one-indexed. Returns 0 for `n` below one.
    #[pyo3(name = "nth")]
    fn __compylr_method_1_3(&mut self, n: i64) -> PyResult<i64> {
        self.inner.nth(n).map_err(__compylr_to_py_err)
    }
}

/// Mean and variance updated one value at a time, by Welford's algorithm.
///
///     The streaming counterpart to `stats.variance`, and the reason to want a class: it never holds
///     the values, so it works over a stream that does not fit in memory — and it is numerically
///     better behaved than summing the squares, which loses precision exactly when the mean is large
///     next to the spread.
///     
#[pyclass(name = "RunningStats")]
pub struct __compylr_class_2 {
    inner: generated::RunningStats,
}

#[pymethods]
impl __compylr_class_2 {
    #[new]
    fn __compylr_init() -> PyResult<Self> {
        Ok(Self {
            inner: generated::RunningStats::__compylr_new().map_err(__compylr_to_py_err)?,
        })
    }

    /// Fold one more observation in.
    #[pyo3(name = "add")]
    fn __compylr_method_2_0(&mut self, value: f64) -> PyResult<()> {
        self.inner.add(value).map_err(__compylr_to_py_err)
    }

    /// The mean so far.
    #[pyo3(name = "mean_value")]
    fn __compylr_method_2_1(&self) -> PyResult<f64> {
        self.inner.mean_value().map_err(__compylr_to_py_err)
    }

    /// How many observations have been folded in.
    #[pyo3(name = "seen")]
    fn __compylr_method_2_2(&self) -> PyResult<i64> {
        self.inner.seen().map_err(__compylr_to_py_err)
    }

    /// The population variance so far. Zero until something has been added.
    #[pyo3(name = "variance_value")]
    fn __compylr_method_2_3(&self) -> PyResult<f64> {
        self.inner.variance_value().map_err(__compylr_to_py_err)
    }
}

/// Disjoint sets, with union by rank and path compression.
///
///     The structure that makes Kruskal's algorithm and connected-components linear-ish, and the
///     best small example of why an instance is not a copy: `find` **rewrites the forest it walks**,
///     and that rewrite has to still be there on the next call or the compression bought nothing.
///     
#[pyclass(name = "UnionFind")]
pub struct __compylr_class_3 {
    inner: generated::UnionFind,
}

#[pymethods]
impl __compylr_class_3 {
    #[new]
    fn __compylr_init(size: i64) -> PyResult<Self> {
        Ok(Self {
            inner: generated::UnionFind::__compylr_new(size).map_err(__compylr_to_py_err)?,
        })
    }

    /// Whether `a` and `b` are in the same set.
    #[pyo3(name = "connected")]
    fn __compylr_method_3_0(&mut self, a: i64, b: i64) -> PyResult<bool> {
        self.inner.connected(a, b).map_err(__compylr_to_py_err)
    }

    /// The representative of `node`'s set, flattening the path to it on the way.
    ///
    ///         Two passes rather than recursion: the root is found first, then every node on the way is
    ///         pointed straight at it. Recursion would be shorter and would put the depth of the tree on
    ///         the call stack, and a stack overflow in compiled code is a process abort with no
    ///         traceback rather than a `RecursionError`.
    ///         
    #[pyo3(name = "find")]
    fn __compylr_method_3_1(&mut self, node: i64) -> PyResult<i64> {
        self.inner.find(node).map_err(__compylr_to_py_err)
    }

    /// How many disjoint sets remain.
    #[pyo3(name = "group_count")]
    fn __compylr_method_3_2(&self) -> PyResult<i64> {
        self.inner.group_count().map_err(__compylr_to_py_err)
    }

    /// Merge the sets containing `a` and `b`. Does nothing when they are already one.
    ///
    ///         Returns `None` so that calling it and ignoring the outcome is a statement the subset
    ///         accepts. `group_count` is how you ask what happened.
    ///         
    #[pyo3(name = "union")]
    fn __compylr_method_3_3(&mut self, a: i64, b: i64) -> PyResult<()> {
        self.inner.union(a, b).map_err(__compylr_to_py_err)
    }
}

#[pyfunction]
#[pyo3(name = "average_of_counts")]
fn __compylr_export_0(counts: Vec<i64>) -> PyResult<f64> {
    generated::average_of_counts(counts).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "balanced")]
fn __compylr_export_1(tokens: Vec<i64>) -> PyResult<bool> {
    generated::balanced(tokens).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "bfs_distances")]
fn __compylr_export_2(graph: FastMap<i64, Vec<i64>>, start: i64) -> PyResult<FastMap<i64, i64>> {
    generated::bfs_distances(graph, start).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "binary_search")]
fn __compylr_export_3(xs: Vec<i64>, target: i64) -> PyResult<i64> {
    generated::binary_search(xs, target).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "coin_change")]
fn __compylr_export_4(coins: Vec<i64>, amount: i64) -> PyResult<i64> {
    generated::coin_change(coins, amount).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "collatz_length")]
fn __compylr_export_5(n: i64) -> PyResult<i64> {
    generated::collatz_length(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "collatz_length_rust")]
fn __compylr_export_6(n: i64) -> PyResult<i64> {
    generated::collatz_length_rust(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "component_count")]
fn __compylr_export_7(size: i64, edges: Vec<(i64, i64)>) -> PyResult<i64> {
    generated::component_count(size, edges).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "copy_of")]
fn __compylr_export_8(xs: Vec<i64>) -> PyResult<Vec<i64>> {
    generated::copy_of(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "count_present")]
fn __compylr_export_9(words: Vec<String>, wanted: FastSet<String>) -> PyResult<i64> {
    generated::count_present(words, wanted).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "depth_first_order")]
fn __compylr_export_10(graph: FastMap<i64, Vec<i64>>, start: i64) -> PyResult<Vec<i64>> {
    generated::depth_first_order(graph, start).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "digit_sum")]
fn __compylr_export_11(n: i64) -> PyResult<i64> {
    generated::digit_sum(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "divide")]
fn __compylr_export_12(a: i64, b: i64) -> PyResult<(i64, i64)> {
    generated::divide(a, b).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "edit_distance")]
fn __compylr_export_13(left: Vec<String>, right: Vec<String>) -> PyResult<i64> {
    generated::edit_distance(left, right).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "extremes")]
fn __compylr_export_14(xs: Vec<f64>) -> PyResult<(f64, f64)> {
    generated::extremes(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "fibonacci")]
fn __compylr_export_15(n: i64) -> PyResult<i64> {
    generated::fibonacci(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "floor_divide")]
fn __compylr_export_16(a: i64, b: i64) -> PyResult<i64> {
    generated::floor_divide(a, b).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "gcd")]
fn __compylr_export_17(a: i64, b: i64) -> PyResult<i64> {
    generated::gcd(a, b).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "has_cycle")]
fn __compylr_export_18(graph: FastMap<i64, Vec<i64>>) -> PyResult<bool> {
    generated::has_cycle(graph).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "identity")]
fn __compylr_export_19(size: i64) -> PyResult<Vec<Vec<i64>>> {
    generated::identity(size).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "insertion_sort")]
fn __compylr_export_20(xs: Vec<i64>) -> PyResult<Vec<i64>> {
    generated::insertion_sort(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "integer_sqrt")]
fn __compylr_export_21(n: i64) -> PyResult<i64> {
    generated::integer_sqrt(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "is_sorted")]
fn __compylr_export_22(xs: Vec<i64>) -> PyResult<bool> {
    generated::is_sorted(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "iterative_not_divisible")]
fn __compylr_export_23(divisible: bool) -> PyResult<bool> {
    generated::iterative_not_divisible(divisible).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "iterative_nth_prime")]
fn __compylr_export_24(n: i64) -> PyResult<i64> {
    generated::iterative_nth_prime(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "iterative_primes_up_to_count")]
fn __compylr_export_25(n: i64) -> PyResult<Vec<i64>> {
    generated::iterative_primes_up_to_count(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "joined")]
fn __compylr_export_26(words: Vec<String>, separator: String) -> PyResult<String> {
    generated::joined(words, separator).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "knapsack")]
fn __compylr_export_27(weights: Vec<i64>, values: Vec<i64>, capacity: i64) -> PyResult<i64> {
    generated::knapsack(weights, values, capacity).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "larger")]
fn __compylr_export_28(a: i64, b: i64) -> PyResult<i64> {
    generated::larger(a, b).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "lcm")]
fn __compylr_export_29(a: i64, b: i64) -> PyResult<i64> {
    generated::lcm(a, b).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "longest")]
fn __compylr_export_30(words: Vec<String>) -> PyResult<String> {
    generated::longest(words).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "longest_common_subsequence")]
fn __compylr_export_31(left: Vec<i64>, right: Vec<i64>) -> PyResult<i64> {
    generated::longest_common_subsequence(left, right).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "mean")]
fn __compylr_export_32(xs: Vec<f64>) -> PyResult<f64> {
    generated::mean(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "median_of_sorted")]
fn __compylr_export_33(xs: Vec<f64>) -> PyResult<f64> {
    generated::median_of_sorted(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "merge")]
fn __compylr_export_34(left: Vec<i64>, right: Vec<i64>) -> PyResult<Vec<i64>> {
    generated::merge(left, right).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "merge_sort")]
fn __compylr_export_35(xs: Vec<i64>) -> PyResult<Vec<i64>> {
    generated::merge_sort(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "missing")]
fn __compylr_export_36(haystack: String, needles: Vec<String>) -> PyResult<Vec<String>> {
    generated::missing(haystack, needles).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "most_common")]
fn __compylr_export_37(words: Vec<String>) -> PyResult<String> {
    generated::most_common(words).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "multiply")]
fn __compylr_export_38(left: Vec<Vec<i64>>, right: Vec<Vec<i64>>) -> PyResult<Vec<Vec<i64>>> {
    generated::multiply(left, right).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "node_list")]
fn __compylr_export_39(graph: FastMap<i64, Vec<i64>>) -> PyResult<Vec<i64>> {
    generated::node_list(graph).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "normalize")]
fn __compylr_export_40(xs: Vec<f64>) -> PyResult<Vec<f64>> {
    generated::normalize(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "occurrences")]
fn __compylr_export_41(haystack: String, needles: Vec<String>) -> PyResult<i64> {
    generated::occurrences(haystack, needles).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "power")]
fn __compylr_export_42(base: i64, exponent: i64) -> PyResult<i64> {
    generated::power(base, exponent).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "recursive_is_prime")]
fn __compylr_export_43(n: i64) -> PyResult<bool> {
    generated::recursive_is_prime(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "recursive_next_prime")]
fn __compylr_export_44(after: i64) -> PyResult<i64> {
    generated::recursive_next_prime(after).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "recursive_nth_prime")]
fn __compylr_export_45(n: i64) -> PyResult<i64> {
    generated::recursive_nth_prime(n).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "recursive_nth_prime_from")]
fn __compylr_export_46(remaining: i64, current: i64) -> PyResult<i64> {
    generated::recursive_nth_prime_from(remaining, current).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "remainder")]
fn __compylr_export_47(a: i64, b: i64) -> PyResult<i64> {
    generated::remainder(a, b).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "row_sums")]
fn __compylr_export_48(matrix: Vec<Vec<i64>>) -> PyResult<Vec<i64>> {
    generated::row_sums(matrix).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "scale")]
fn __compylr_export_49(matrix: Vec<Vec<i64>>, factor: i64) -> PyResult<Vec<Vec<i64>>> {
    generated::scale(matrix, factor).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "selection_sort")]
fn __compylr_export_50(xs: Vec<i64>) -> PyResult<Vec<i64>> {
    generated::selection_sort(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "sieve")]
fn __compylr_export_51(limit: i64) -> PyResult<Vec<i64>> {
    generated::sieve(limit).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "smaller")]
fn __compylr_export_52(a: i64, b: i64) -> PyResult<i64> {
    generated::smaller(a, b).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "square_root")]
fn __compylr_export_53(value: f64) -> PyResult<f64> {
    generated::square_root(value).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "standard_deviation")]
fn __compylr_export_54(xs: Vec<f64>) -> PyResult<f64> {
    generated::standard_deviation(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "table_of_zeros")]
fn __compylr_export_55(rows: i64, columns: i64) -> PyResult<Vec<Vec<i64>>> {
    generated::table_of_zeros(rows, columns).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "to_base")]
fn __compylr_export_56(n: i64, base: i64) -> PyResult<Vec<i64>> {
    generated::to_base(n, base).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "topological_order")]
fn __compylr_export_57(graph: FastMap<i64, Vec<i64>>) -> PyResult<Vec<i64>> {
    generated::topological_order(graph).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "total_length")]
fn __compylr_export_58(words: Vec<String>) -> PyResult<i64> {
    generated::total_length(words).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "trace")]
fn __compylr_export_59(matrix: Vec<Vec<i64>>) -> PyResult<i64> {
    generated::trace(matrix).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "transpose")]
fn __compylr_export_60(matrix: Vec<Vec<i64>>) -> PyResult<Vec<Vec<i64>>> {
    generated::transpose(matrix).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "unique_words")]
fn __compylr_export_61(words: Vec<String>) -> PyResult<Vec<String>> {
    generated::unique_words(words).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "variance")]
fn __compylr_export_62(xs: Vec<f64>) -> PyResult<f64> {
    generated::variance(xs).map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "vowel_letters")]
fn __compylr_export_63() -> PyResult<FastSet<String>> {
    generated::vowel_letters().map_err(__compylr_to_py_err)
}

#[pyfunction]
#[pyo3(name = "word_count")]
fn __compylr_export_64(words: Vec<String>) -> PyResult<FastMap<String, i64>> {
    generated::word_count(words).map_err(__compylr_to_py_err)
}

/// Register every compiled function on the module.
///
/// Kept here rather than in the crate root so that `wrap_pyfunction!` resolves the
/// wrappers locally, and so the root stays the same size for any number of functions.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<__compylr_class_0>()?;
    m.add_class::<__compylr_class_1>()?;
    m.add_class::<__compylr_class_2>()?;
    m.add_class::<__compylr_class_3>()?;
    m.add_function(wrap_pyfunction!(__compylr_export_0, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_1, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_2, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_3, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_4, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_5, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_6, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_7, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_8, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_9, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_10, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_11, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_12, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_13, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_14, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_15, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_16, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_17, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_18, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_19, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_20, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_21, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_22, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_23, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_24, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_25, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_26, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_27, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_28, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_29, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_30, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_31, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_32, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_33, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_34, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_35, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_36, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_37, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_38, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_39, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_40, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_41, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_42, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_43, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_44, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_45, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_46, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_47, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_48, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_49, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_50, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_51, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_52, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_53, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_54, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_55, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_56, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_57, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_58, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_59, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_60, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_61, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_62, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_63, m)?)?;
    m.add_function(wrap_pyfunction!(__compylr_export_64, m)?)?;
    Ok(())
}
