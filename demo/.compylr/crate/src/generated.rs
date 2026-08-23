//! Translated by compylr.

use crate::compat::{
    FastMap, FastSet, IndexOrigin, PyAdd, PyAddAssign, PyContains, PyIterate, PyLen, PyNum,
    PySetItem, RuntimeError, TextUnits, div_exact, py_borrow, py_place, py_subscript,
};

/// A stack of integers, over a list that never shrinks.
///
///     `append` is the only collection method in the subset — there is no `pop` — so the stack is a
///     list and a `height`. Pushing writes over a slot past the height when one is there and appends
///     otherwise, and popping just lowers the height. The list therefore grows to the deepest the
///     stack ever was and stays there, which is the allocation `pop` would have handed back and that
///     a stack is about to want again anyway.
///     
#[derive(Clone)]
pub struct IntStack {
    pub slots: Vec<i64>,
    pub height: i64,
}

impl IntStack {
    pub fn __compylr_new() -> Result<Self, RuntimeError> {
        let slots: Vec<i64> = vec![];
        let height: i64 = 0i64;
        Ok(Self { slots, height })
    }

    /// How many values are on the stack.
    pub fn depth(&self) -> Result<i64, RuntimeError> {
        Ok((self).height.clone())
    }

    /// The top value without removing it, or 0 when the stack is empty.
    pub fn peek(&self) -> Result<i64, RuntimeError> {
        if ((&((self).height.clone())) == (&(0i64))) {
            return Ok(0i64);
        }
        Ok(py_subscript(
            &((self).slots),
            &(PyNum::py_sub(&((self).height.clone()), &(1i64))?),
            IndexOrigin::FromEitherEnd,
        )?)
    }

    /// Take the top value off, or 0 when the stack is empty.
    pub fn pop(&mut self) -> Result<i64, RuntimeError> {
        if ((&((self).height.clone())) == (&(0i64))) {
            return Ok(0i64);
        }
        (self).height = PyNum::py_sub(&((self).height.clone()), &(1i64))?;
        Ok(py_subscript(
            &((self).slots),
            &((self).height.clone()),
            IndexOrigin::FromEitherEnd,
        )?)
    }

    /// Put `value` on top.
    pub fn push(&mut self, value: i64) -> Result<(), RuntimeError> {
        if ((&((self).height.clone())) < (&(PyLen::py_len(&((self).slots), TextUnits::CodePoints))))
        {
            {
                let __compylr_value = value.clone();
                let __compylr_index = (self).height.clone();
                PySetItem::py_set(&mut ((self).slots), &__compylr_index, __compylr_value)?;
            }
        } else {
            {
                let __compylr_value = value.clone();
                ((self).slots).push(__compylr_value);
            }
        }
        (self).height = PyAdd::py_add(&((self).height.clone()), &(1i64))?;
        Ok(())
    }
}

/// An nth-prime that remembers what it has already computed.
#[derive(Clone)]
pub struct PrimeCache {
    pub known: FastMap<i64, i64>,
    pub hits: i64,
}

impl PrimeCache {
    pub fn __compylr_new() -> Result<Self, RuntimeError> {
        let known: FastMap<i64, i64> = FastMap::from_iter([]);
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
            PyAddAssign::py_add_assign(&mut d, &(1i64))?;
        }
        Ok(true)
    }

    /// How many answers are cached.
    pub fn known_count(&self) -> Result<i64, RuntimeError> {
        Ok(PyLen::py_len(&((self).known), TextUnits::CodePoints))
    }

    /// The `n`th prime, one-indexed. Returns 0 for `n` below one.
    pub fn nth(&mut self, n: i64) -> Result<i64, RuntimeError> {
        if ((&(n)) < (&(1i64))) {
            return Ok(0i64);
        }
        if PyContains::py_contains(&((self).known.clone()), &(n)) {
            (self).hits = PyAdd::py_add(&((self).hits.clone()), &(1i64))?;
            return Ok(py_subscript(
                &((self).known),
                &(n),
                IndexOrigin::FromEitherEnd,
            )?);
        }
        let mut found: i64 = 0i64;
        let mut candidate: i64 = 1i64;
        while ((&(found)) < (&(n))) {
            PyAddAssign::py_add_assign(&mut candidate, &(1i64))?;
            if (self).is_prime(candidate)? {
                PyAddAssign::py_add_assign(&mut found, &(1i64))?;
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

/// Mean and variance updated one value at a time, by Welford's algorithm.
///
///     The streaming counterpart to `stats.variance`, and the reason to want a class: it never holds
///     the values, so it works over a stream that does not fit in memory — and it is numerically
///     better behaved than summing the squares, which loses precision exactly when the mean is large
///     next to the spread.
///     
#[derive(Clone)]
pub struct RunningStats {
    pub count: i64,
    pub average: f64,
    pub sum_of_squares: f64,
}

impl RunningStats {
    pub fn __compylr_new() -> Result<Self, RuntimeError> {
        let count: i64 = 0i64;
        let average: f64 = 0.0f64;
        let sum_of_squares: f64 = 0.0f64;
        Ok(Self {
            count,
            average,
            sum_of_squares,
        })
    }

    /// Fold one more observation in.
    pub fn add(&mut self, value: f64) -> Result<(), RuntimeError> {
        (self).count = PyAdd::py_add(&((self).count.clone()), &(1i64))?;
        let delta: f64 = PyNum::py_sub(&(value), &((self).average.clone()))?;
        (self).average = PyAdd::py_add(
            &((self).average.clone()),
            &(div_exact(&(delta), &(((self).count.clone()) as f64))?),
        )?;
        (self).sum_of_squares = PyAdd::py_add(
            &((self).sum_of_squares.clone()),
            &(PyNum::py_mul(
                &(delta),
                &(PyNum::py_sub(&(value), &((self).average.clone()))?),
            )?),
        )?;
        Ok(())
    }

    /// The mean so far.
    pub fn mean_value(&self) -> Result<f64, RuntimeError> {
        Ok((self).average.clone())
    }

    /// How many observations have been folded in.
    pub fn seen(&self) -> Result<i64, RuntimeError> {
        Ok((self).count.clone())
    }

    /// The population variance so far. Zero until something has been added.
    pub fn variance_value(&self) -> Result<f64, RuntimeError> {
        if ((&((self).count.clone())) == (&(0i64))) {
            return Ok(0.0f64);
        }
        Ok(div_exact(
            &((self).sum_of_squares.clone()),
            &(((self).count.clone()) as f64),
        )?)
    }
}

/// Disjoint sets, with union by rank and path compression.
///
///     The structure that makes Kruskal's algorithm and connected-components linear-ish, and the
///     best small example of why an instance is not a copy: `find` **rewrites the forest it walks**,
///     and that rewrite has to still be there on the next call or the compression bought nothing.
///     
#[derive(Clone)]
pub struct UnionFind {
    pub parent: Vec<i64>,
    pub rank: Vec<i64>,
    pub groups: i64,
}

impl UnionFind {
    pub fn __compylr_new(size: i64) -> Result<Self, RuntimeError> {
        let mut parent: Vec<i64> = vec![];
        let mut rank: Vec<i64> = vec![];
        let groups: i64 = size;
        {
            let __compylr_stop: i64 = size;
            let __compylr_step: i64 = 1i64;
            if __compylr_step == 0 {
                return Err(RuntimeError::ZeroStep);
            }
            let mut __compylr_cursor: i64 = 0i64;
            while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
                || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
            {
                let node: i64 = __compylr_cursor;
                __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
                {
                    let __compylr_value = node.clone();
                    (parent).push(__compylr_value);
                }
                {
                    let __compylr_value = 0i64;
                    (rank).push(__compylr_value);
                }
            }
        }
        Ok(Self {
            parent,
            rank,
            groups,
        })
    }

    /// Whether `a` and `b` are in the same set.
    pub fn connected(&mut self, a: i64, b: i64) -> Result<bool, RuntimeError> {
        Ok(((&((self).find(a)?)) == (&((self).find(b)?))))
    }

    /// The representative of `node`'s set, flattening the path to it on the way.
    ///
    ///         Two passes rather than recursion: the root is found first, then every node on the way is
    ///         pointed straight at it. Recursion would be shorter and would put the depth of the tree on
    ///         the call stack, and a stack overflow in compiled code is a process abort with no
    ///         traceback rather than a `RecursionError`.
    ///         
    pub fn find(&mut self, node: i64) -> Result<i64, RuntimeError> {
        let mut root: i64 = node;
        while ((&(py_subscript(&((self).parent), &(root), IndexOrigin::FromEitherEnd)?))
            != (&(root)))
        {
            root = py_subscript(&((self).parent), &(root), IndexOrigin::FromEitherEnd)?;
        }
        let mut current: i64 = node;
        while ((&(py_subscript(&((self).parent), &(current), IndexOrigin::FromEitherEnd)?))
            != (&(current)))
        {
            let following: i64 =
                py_subscript(&((self).parent), &(current), IndexOrigin::FromEitherEnd)?;
            {
                let __compylr_value = root.clone();
                let __compylr_index = current.clone();
                PySetItem::py_set(&mut ((self).parent), &__compylr_index, __compylr_value)?;
            }
            current = following;
        }
        Ok(root)
    }

    /// How many disjoint sets remain.
    pub fn group_count(&self) -> Result<i64, RuntimeError> {
        Ok((self).groups.clone())
    }

    /// Merge the sets containing `a` and `b`. Does nothing when they are already one.
    ///
    ///         Returns `None` so that calling it and ignoring the outcome is a statement the subset
    ///         accepts. `group_count` is how you ask what happened.
    ///         
    pub fn union(&mut self, a: i64, b: i64) -> Result<(), RuntimeError> {
        let mut left: i64 = (self).find(a)?;
        let mut right: i64 = (self).find(b)?;
        if ((&(left)) == (&(right))) {
            return Ok(());
        }
        if ((&(py_subscript(&((self).rank), &(left), IndexOrigin::FromEitherEnd)?))
            < (&(py_subscript(&((self).rank), &(right), IndexOrigin::FromEitherEnd)?)))
        {
            let held: i64 = left;
            left = right;
            right = held;
        }
        {
            let __compylr_value = left.clone();
            let __compylr_index = right.clone();
            PySetItem::py_set(&mut ((self).parent), &__compylr_index, __compylr_value)?;
        }
        if ((&(py_subscript(&((self).rank), &(left), IndexOrigin::FromEitherEnd)?))
            == (&(py_subscript(&((self).rank), &(right), IndexOrigin::FromEitherEnd)?)))
        {
            {
                let __compylr_value = PyAdd::py_add(
                    &(py_subscript(&((self).rank), &(left), IndexOrigin::FromEitherEnd)?),
                    &(1i64),
                )?;
                let __compylr_index = left.clone();
                PySetItem::py_set(&mut ((self).rank), &__compylr_index, __compylr_value)?;
            }
        }
        (self).groups = PyNum::py_sub(&((self).groups.clone()), &(1i64))?;
        Ok(())
    }
}

/// The mean of a list of **integers**, as a float.
///
///     The one-line demonstration that `/` is exact division: `total` and `len(counts)` are both
///     integers, and the result is not. Both operands are widened, and the widening is visible in
///     `.compylr/ir/unit.json` as a `ToFloat` node wrapping each of them.
///     
pub fn average_of_counts(counts: Vec<i64>) -> Result<f64, RuntimeError> {
    if ((&(PyLen::py_len(&(counts), TextUnits::CodePoints))) == (&(0i64))) {
        return Ok(0.0f64);
    }
    let mut total: i64 = 0i64;
    {
        let __compylr_iter = &counts;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let count: i64 = __compylr_item;
            PyAddAssign::py_add_assign(&mut total, &(count))?;
        }
    }
    Ok(div_exact(
        &((total) as f64),
        &((PyLen::py_len(&(counts), TextUnits::CodePoints)) as f64),
    )?)
}

/// Whether the markers in `tokens` open and close in the right order.
///
///     A positive token opens, and its negative closes it: `[1, 2, -2, -1]` is balanced and
///     `[1, 2, -1, -2]` is not. Integers rather than characters because a `str` cannot be indexed or
///     iterated in the subset, so a bracket string would have to be tokenised by the caller anyway.
///
///     `stack.push(token)` is a statement rather than an expression, which the subset allows only
///     because `push` returns `None`. Discarding a *value* is rejected — it is either dead code or a
///     side effect the subset cannot express — so a method whose result you mean to ignore has to
///     say so in its return type.
///     
pub fn balanced(tokens: Vec<i64>) -> Result<bool, RuntimeError> {
    let mut stack: IntStack = IntStack::__compylr_new()?;
    {
        let __compylr_iter = &tokens;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let token: i64 = __compylr_item;
            if ((&(token)) > (&(0i64))) {
                (stack).push(token)?;
            } else {
                if ((&((stack).depth()?)) == (&(0i64))) {
                    return Ok(false);
                }
                if ((&(PyAdd::py_add(&((stack).pop()?), &(token))?)) != (&(0i64))) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(((&((stack).depth()?)) == (&(0i64))))
}

/// How many hops each reachable node is from `start`.
///
///     The queue is a list and a `head` cursor. Nothing is ever removed, so the list is also the
///     visit order if you want it — and the traversal never pays `pop(0)`'s cost of shifting every
///     remaining element down by one.
///     
pub fn bfs_distances(
    graph: FastMap<i64, Vec<i64>>,
    start: i64,
) -> Result<FastMap<i64, i64>, RuntimeError> {
    let mut distance: FastMap<i64, i64> = FastMap::from_iter([]);
    {
        let __compylr_value = 0i64;
        let __compylr_index = start.clone();
        PySetItem::py_set(&mut (distance), &__compylr_index, __compylr_value)?;
    }
    let mut queue: Vec<i64> = vec![];
    {
        let __compylr_value = start.clone();
        (queue).push(__compylr_value);
    }
    let mut head: i64 = 0i64;
    while ((&(head)) < (&(PyLen::py_len(&(queue), TextUnits::CodePoints)))) {
        let node: i64 = py_subscript(&(queue), &(head), IndexOrigin::FromEitherEnd)?;
        PyAddAssign::py_add_assign(&mut head, &(1i64))?;
        if !(PyContains::py_contains(&(graph), &(node))) {
            continue;
        }
        {
            let __compylr_iter = &(*py_borrow(&(graph), &(node), IndexOrigin::FromEitherEnd)?);
            for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                let neighbour: i64 = __compylr_item;
                if PyContains::py_contains(&(distance), &(neighbour)) {
                    continue;
                }
                {
                    let __compylr_value = PyAdd::py_add(
                        &(py_subscript(&(distance), &(node), IndexOrigin::FromEitherEnd)?),
                        &(1i64),
                    )?;
                    let __compylr_index = neighbour.clone();
                    PySetItem::py_set(&mut (distance), &__compylr_index, __compylr_value)?;
                }
                {
                    let __compylr_value = neighbour.clone();
                    (queue).push(__compylr_value);
                }
            }
        }
    }
    Ok(distance)
}

/// The index of `target` in the ascending `xs`, or -1 when it is absent.
///
///     -1 rather than an exception: the compiled subset has no exceptions of its own, so a sentinel
///     is the only answer available. It is documented rather than discovered, and `-1` is also a
///     valid index into a list, which is why the caller must treat it as "absent" and not index with
///     it — `xs[-1]` counts from the end here exactly as it does in Python.
///     
pub fn binary_search(xs: Vec<i64>, target: i64) -> Result<i64, RuntimeError> {
    let mut low: i64 = 0i64;
    let mut high: i64 = PyNum::py_sub(&(PyLen::py_len(&(xs), TextUnits::CodePoints)), &(1i64))?;
    let mut found: i64 = -1i64;
    while ((&(low)) <= (&(high))) {
        let middle: i64 = PyNum::div_floor(&(PyAdd::py_add(&(low), &(high))?), &(2i64))?;
        if ((&(py_subscript(&(xs), &(middle), IndexOrigin::FromEitherEnd)?)) == (&(target))) {
            found = middle;
            break;
        }
        if ((&(py_subscript(&(xs), &(middle), IndexOrigin::FromEitherEnd)?)) < (&(target))) {
            low = PyAdd::py_add(&(middle), &(1i64))?;
        } else {
            high = PyNum::py_sub(&(middle), &(1i64))?;
        }
    }
    Ok(found)
}

/// The fewest coins that make `amount`, or -1 when no combination does.
///
///     A one-dimensional table, and the place a sentinel earns its keep: "unreachable" has to be a
///     value the arithmetic cannot accidentally produce, so it is `amount + 1` — one more than the
///     largest number of coins any real answer could use.
///     
pub fn coin_change(coins: Vec<i64>, amount: i64) -> Result<i64, RuntimeError> {
    if ((&(amount)) < (&(0i64))) {
        return Ok(-1i64);
    }
    let unreachable: i64 = PyAdd::py_add(&(amount), &(1i64))?;
    let mut best: Vec<i64> = vec![];
    {
        let __compylr_stop: i64 = PyAdd::py_add(&(amount), &(1i64))?;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let _slot: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_value = unreachable.clone();
                (best).push(__compylr_value);
            }
        }
    }
    {
        let __compylr_value = 0i64;
        let __compylr_index = 0i64;
        PySetItem::py_set(&mut (best), &__compylr_index, __compylr_value)?;
    }
    {
        let __compylr_stop: i64 = PyAdd::py_add(&(amount), &(1i64))?;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 1i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let target: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_iter = &coins;
                for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                    let coin: i64 = __compylr_item;
                    if ((&(coin)) > (&(target))) {
                        continue;
                    }
                    let candidate: i64 = PyAdd::py_add(
                        &(py_subscript(
                            &(best),
                            &(PyNum::py_sub(&(target), &(coin))?),
                            IndexOrigin::FromEitherEnd,
                        )?),
                        &(1i64),
                    )?;
                    if ((&(candidate))
                        < (&(py_subscript(&(best), &(target), IndexOrigin::FromEitherEnd)?)))
                    {
                        {
                            let __compylr_value = candidate.clone();
                            let __compylr_index = target.clone();
                            PySetItem::py_set(&mut (best), &__compylr_index, __compylr_value)?;
                        }
                    }
                }
            }
        }
    }
    if ((&(py_subscript(&(best), &(amount), IndexOrigin::FromEitherEnd)?)) == (&(unreachable))) {
        return Ok(-1i64);
    }
    Ok(py_subscript(
        &(best),
        &(amount),
        IndexOrigin::FromEitherEnd,
    )?)
}

/// How many steps `n` takes to reach 1 under the Collatz rule. Zero for `n` below one.
///
///     Nobody has proved this terminates for every `n`. It is in the demo because it is the shortest
///     honest example of a loop whose trip count is not a function of its input's size.
///     
pub fn collatz_length(n: i64) -> Result<i64, RuntimeError> {
    if ((&(n)) < (&(1i64))) {
        return Ok(0i64);
    }
    let mut steps: i64 = 0i64;
    let mut current: i64 = n;
    while ((&(current)) != (&(1i64))) {
        if ((&(PyNum::rem_floor(&(current), &(2i64))?)) == (&(0i64))) {
            current = PyNum::div_floor(&(current), &(2i64))?;
        } else {
            current = PyAdd::py_add(&(PyNum::py_mul(&(3i64), &(current))?), &(1i64))?;
        }
        PyAddAssign::py_add_assign(&mut steps, &(1i64))?;
    }
    Ok(steps)
}

/// How many connected components `size` nodes fall into, given `edges`.
///
///     Edges are `tuple[int, int]`, so each one is read by position: `edge[0]` and `edge[1]`. The
///     position has to be a literal — a tuple is typed per position, so `edge[i]` for a computed `i`
///     would have a type that depends on a runtime value, and is rejected.
///     
pub fn component_count(size: i64, edges: Vec<(i64, i64)>) -> Result<i64, RuntimeError> {
    let mut sets: UnionFind = UnionFind::__compylr_new(size)?;
    {
        let __compylr_iter = &edges;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let edge: &(i64, i64) = __compylr_item;
            (sets).union((edge).0.clone(), (edge).1.clone())?;
        }
    }
    Ok((sets).group_count()?)
}

/// A fresh list with the same elements.
///
///     The first line of every sort here. `out = xs` would bind a second name to the same list in
///     Python and copy in compylr, so mutating it is the same hazard one line further out —
///     compylr rejects that transitively rather than letting the two languages disagree silently.
///     
pub fn copy_of(xs: Vec<i64>) -> Result<Vec<i64>, RuntimeError> {
    let mut out: Vec<i64> = vec![];
    {
        let __compylr_iter = &xs;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let x: i64 = __compylr_item;
            {
                let __compylr_value = x.clone();
                (out).push(__compylr_value);
            }
        }
    }
    Ok(out)
}

/// How many of `words` are in `wanted`.
///
///     The set is the point: this is a hash lookup per word rather than a scan of a list, and the
///     difference is the whole reason to hand a set across the boundary instead of a list.
///     
pub fn count_present(words: Vec<String>, wanted: FastSet<String>) -> Result<i64, RuntimeError> {
    let mut total: i64 = 0i64;
    {
        let __compylr_iter = &words;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let word: &String = __compylr_item;
            if PyContains::py_contains(&(wanted), &(word)) {
                PyAddAssign::py_add_assign(&mut total, &(1i64))?;
            }
        }
    }
    Ok(total)
}

/// Nodes in the order a depth-first traversal first reaches them.
///
///     The stack is a list and a `top` index: pushing writes over a slot that is past `top` if one
///     exists and appends otherwise, and popping just moves `top` down. The list therefore only ever
///     grows to the deepest the stack ever was, which is the allocation `pop` would have given back.
///
///     Neighbours go on in reverse so the first one listed is the first one visited, which is what
///     the recursive version does and what a reader will expect.
///     
pub fn depth_first_order(
    graph: FastMap<i64, Vec<i64>>,
    start: i64,
) -> Result<Vec<i64>, RuntimeError> {
    let mut order: Vec<i64> = vec![];
    let mut seen: FastMap<i64, i64> = FastMap::from_iter([]);
    let mut stack: Vec<i64> = vec![];
    {
        let __compylr_value = start.clone();
        (stack).push(__compylr_value);
    }
    let mut top: i64 = 1i64;
    while ((&(top)) > (&(0i64))) {
        top = PyNum::py_sub(&(top), &(1i64))?;
        let node: i64 = py_subscript(&(stack), &(top), IndexOrigin::FromEitherEnd)?;
        if PyContains::py_contains(&(seen), &(node)) {
            continue;
        }
        {
            let __compylr_value = 1i64;
            let __compylr_index = node.clone();
            PySetItem::py_set(&mut (seen), &__compylr_index, __compylr_value)?;
        }
        {
            let __compylr_value = node.clone();
            (order).push(__compylr_value);
        }
        if !(PyContains::py_contains(&(graph), &(node))) {
            continue;
        }
        let neighbours: Vec<i64> = py_subscript(&(graph), &(node), IndexOrigin::FromEitherEnd)?;
        let mut index: i64 = PyNum::py_sub(
            &(PyLen::py_len(&(neighbours), TextUnits::CodePoints)),
            &(1i64),
        )?;
        while ((&(index)) >= (&(0i64))) {
            if ((&(top)) < (&(PyLen::py_len(&(stack), TextUnits::CodePoints)))) {
                {
                    let __compylr_value =
                        py_subscript(&(neighbours), &(index), IndexOrigin::FromEitherEnd)?;
                    let __compylr_index = top.clone();
                    PySetItem::py_set(&mut (stack), &__compylr_index, __compylr_value)?;
                }
            } else {
                {
                    let __compylr_value =
                        py_subscript(&(neighbours), &(index), IndexOrigin::FromEitherEnd)?;
                    (stack).push(__compylr_value);
                }
            }
            PyAddAssign::py_add_assign(&mut top, &(1i64))?;
            index = PyNum::py_sub(&(index), &(1i64))?;
        }
    }
    Ok(order)
}

/// The sum of the decimal digits of `n`, ignoring its sign.
pub fn digit_sum(n: i64) -> Result<i64, RuntimeError> {
    let mut current: i64 = n;
    if ((&(current)) < (&(0i64))) {
        current = PyNum::py_neg(&(current))?;
    }
    let mut total: i64 = 0i64;
    while ((&(current)) > (&(0i64))) {
        PyAddAssign::py_add_assign(&mut total, &(PyNum::rem_floor(&(current), &(10i64))?))?;
        current = PyNum::div_floor(&(current), &(10i64))?;
    }
    Ok(total)
}

/// Quotient and remainder together — Python's `divmod`.
///
///     A tuple is the subset's only heterogeneous value, and the only way a compiled function
///     returns two things. Its positions are typed independently, so reading one is resolved at
///     compile time: `pair[i]` for a computed `i` is rejected, because the result's type would
///     depend on a runtime value.
///     
pub fn divide(a: i64, b: i64) -> Result<(i64, i64), RuntimeError> {
    Ok((PyNum::div_floor(&(a), &(b))?, PyNum::rem_floor(&(a), &(b))?))
}

/// The Levenshtein distance between two sequences of tokens.
///
///     Over `list[str]` rather than over two strings, because a `str` cannot be indexed in the
///     subset — see `text.py`. The algorithm is identical; only the unit of comparison moves from a
///     character to a word.
///     
pub fn edit_distance(left: Vec<String>, right: Vec<String>) -> Result<i64, RuntimeError> {
    let rows: i64 = PyLen::py_len(&(left), TextUnits::CodePoints);
    let columns: i64 = PyLen::py_len(&(right), TextUnits::CodePoints);
    let mut table: Vec<Vec<i64>> = table_of_zeros(
        PyAdd::py_add(&(rows), &(1i64))?,
        PyAdd::py_add(&(columns), &(1i64))?,
    )?;
    {
        let __compylr_stop: i64 = PyAdd::py_add(&(rows), &(1i64))?;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let i: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_value = i.clone();
                let __compylr_index = 0i64;
                PySetItem::py_set(
                    &mut (*py_place(&mut (table), &(i), IndexOrigin::FromEitherEnd)?),
                    &__compylr_index,
                    __compylr_value,
                )?;
            }
        }
    }
    {
        let __compylr_stop: i64 = PyAdd::py_add(&(columns), &(1i64))?;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let j: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_value = j.clone();
                let __compylr_index = j.clone();
                PySetItem::py_set(
                    &mut (*py_place(&mut (table), &(0i64), IndexOrigin::FromEitherEnd)?),
                    &__compylr_index,
                    __compylr_value,
                )?;
            }
        }
    }
    {
        let __compylr_stop: i64 = PyAdd::py_add(&(rows), &(1i64))?;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 1i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let i: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_stop: i64 = PyAdd::py_add(&(columns), &(1i64))?;
                let __compylr_step: i64 = 1i64;
                if __compylr_step == 0 {
                    return Err(RuntimeError::ZeroStep);
                }
                let mut __compylr_cursor: i64 = 1i64;
                while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
                    || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
                {
                    let j: i64 = __compylr_cursor;
                    __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
                    if ((&(py_subscript(
                        &(left),
                        &(PyNum::py_sub(&(i), &(1i64))?),
                        IndexOrigin::FromEitherEnd,
                    )?)) == (&(py_subscript(
                        &(right),
                        &(PyNum::py_sub(&(j), &(1i64))?),
                        IndexOrigin::FromEitherEnd,
                    )?))) {
                        {
                            let __compylr_value = py_subscript(
                                &(*py_borrow(
                                    &(table),
                                    &(PyNum::py_sub(&(i), &(1i64))?),
                                    IndexOrigin::FromEitherEnd,
                                )?),
                                &(PyNum::py_sub(&(j), &(1i64))?),
                                IndexOrigin::FromEitherEnd,
                            )?;
                            let __compylr_index = j.clone();
                            PySetItem::py_set(
                                &mut (*py_place(&mut (table), &(i), IndexOrigin::FromEitherEnd)?),
                                &__compylr_index,
                                __compylr_value,
                            )?;
                        }
                    } else {
                        let best: i64 = smaller(
                            py_subscript(
                                &(*py_borrow(
                                    &(table),
                                    &(PyNum::py_sub(&(i), &(1i64))?),
                                    IndexOrigin::FromEitherEnd,
                                )?),
                                &(j),
                                IndexOrigin::FromEitherEnd,
                            )?,
                            py_subscript(
                                &(*py_borrow(&(table), &(i), IndexOrigin::FromEitherEnd)?),
                                &(PyNum::py_sub(&(j), &(1i64))?),
                                IndexOrigin::FromEitherEnd,
                            )?,
                        )?;
                        {
                            let __compylr_value = PyAdd::py_add(
                                &(smaller(
                                    best,
                                    py_subscript(
                                        &(*py_borrow(
                                            &(table),
                                            &(PyNum::py_sub(&(i), &(1i64))?),
                                            IndexOrigin::FromEitherEnd,
                                        )?),
                                        &(PyNum::py_sub(&(j), &(1i64))?),
                                        IndexOrigin::FromEitherEnd,
                                    )?,
                                )?),
                                &(1i64),
                            )?;
                            let __compylr_index = j.clone();
                            PySetItem::py_set(
                                &mut (*py_place(&mut (table), &(i), IndexOrigin::FromEitherEnd)?),
                                &__compylr_index,
                                __compylr_value,
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(py_subscript(
        &(*py_borrow(&(table), &(rows), IndexOrigin::FromEitherEnd)?),
        &(columns),
        IndexOrigin::FromEitherEnd,
    )?)
}

/// The smallest and largest values, together. `(0.0, 0.0)` for an empty list.
///
///     `min` and `max` are not in the subset — only `len` and `range` are builtins — so this is the
///     loop they would have hidden. Returning both from one pass is what the tuple is for.
///     
pub fn extremes(xs: Vec<f64>) -> Result<(f64, f64), RuntimeError> {
    if ((&(PyLen::py_len(&(xs), TextUnits::CodePoints))) == (&(0i64))) {
        return Ok((0.0f64, 0.0f64));
    }
    let mut smallest: f64 = py_subscript(&(xs), &(0i64), IndexOrigin::FromEitherEnd)?;
    let mut largest: f64 = py_subscript(&(xs), &(0i64), IndexOrigin::FromEitherEnd)?;
    {
        let __compylr_iter = &xs;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let x: f64 = __compylr_item;
            if ((&(x)) < (&(smallest))) {
                smallest = x;
            }
            if ((&(x)) > (&(largest))) {
                largest = x;
            }
        }
    }
    Ok((smallest, largest))
}

/// The `n`th Fibonacci number, iteratively. Zero for a negative `n`.
///
///     The bottom-up version rather than the recursive one on purpose: the recursion is exponential
///     in both languages, so timing it would compare two implementations of the same waste. What is
///     worth measuring is the loop.
///     
pub fn fibonacci(n: i64) -> Result<i64, RuntimeError> {
    if ((&(n)) < (&(0i64))) {
        return Ok(0i64);
    }
    let mut previous: i64 = 0i64;
    let mut current: i64 = 1i64;
    let mut step: i64 = 0i64;
    while ((&(step)) < (&(n))) {
        let held: i64 = current;
        current = PyAdd::py_add(&(previous), &(current))?;
        previous = held;
        PyAddAssign::py_add_assign(&mut step, &(1i64))?;
    }
    Ok(previous)
}

/// `a // b`, rounding toward negative infinity as Python does.
///
///     One line, and the reason the IR carries a rounding mode: `-7 // 2` is `-4`, where Rust's
///     native `/` gives `-3`. Nothing about the operator's *name* says which, so a backend that
///     matched on the name would be silently wrong for a frontend that meant the other one.
///     
pub fn floor_divide(a: i64, b: i64) -> Result<i64, RuntimeError> {
    Ok(PyNum::div_floor(&(a), &(b))?)
}

/// The greatest common divisor, by Euclid's algorithm.
///
///     Both arguments are made non-negative first, which is why this never depends on how `%` signs
///     its result — `floor_divide` and `remainder` are where that shows.
///     
pub fn gcd(a: i64, b: i64) -> Result<i64, RuntimeError> {
    let mut x: i64 = a;
    let mut y: i64 = b;
    if ((&(x)) < (&(0i64))) {
        x = PyNum::py_neg(&(x))?;
    }
    if ((&(y)) < (&(0i64))) {
        y = PyNum::py_neg(&(y))?;
    }
    while ((&(y)) != (&(0i64))) {
        let held: i64 = y;
        y = PyNum::rem_floor(&(x), &(y))?;
        x = held;
    }
    Ok(x)
}

/// Whether the graph contains a directed cycle.
///
///     Asked of `topological_order`'s result rather than by a second traversal: a graph orders
///     completely exactly when it is acyclic, so one implementation answers both questions and the
///     two can never disagree.
///     
pub fn has_cycle(graph: FastMap<i64, Vec<i64>>) -> Result<bool, RuntimeError> {
    Ok(
        ((&(PyLen::py_len(&(topological_order(graph.clone())?), TextUnits::CodePoints)))
            < (&(PyLen::py_len(&(node_list(graph.clone())?), TextUnits::CodePoints)))),
    )
}

/// The `size` by `size` identity matrix.
pub fn identity(size: i64) -> Result<Vec<Vec<i64>>, RuntimeError> {
    let mut out: Vec<Vec<i64>> = table_of_zeros(size, size)?;
    {
        let __compylr_stop: i64 = size;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let i: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_value = 1i64;
                let __compylr_index = i.clone();
                PySetItem::py_set(
                    &mut (*py_place(&mut (out), &(i), IndexOrigin::FromEitherEnd)?),
                    &__compylr_index,
                    __compylr_value,
                )?;
            }
        }
    }
    Ok(out)
}

/// `xs` in ascending order, by insertion sort.
///
///     O(n^2), and the one worth reading: the inner loop is where `and` would normally go.
///     `while j >= 0 and out[j] > key` has no spelling here, so the second half of the condition
///     becomes an `if` with a `break`. Same loop, one line longer.
///     
pub fn insertion_sort(xs: Vec<i64>) -> Result<Vec<i64>, RuntimeError> {
    let mut out: Vec<i64> = copy_of(xs.clone())?;
    let mut i: i64 = 1i64;
    while ((&(i)) < (&(PyLen::py_len(&(out), TextUnits::CodePoints)))) {
        let key: i64 = py_subscript(&(out), &(i), IndexOrigin::FromEitherEnd)?;
        let mut j: i64 = PyNum::py_sub(&(i), &(1i64))?;
        while ((&(j)) >= (&(0i64))) {
            if ((&(py_subscript(&(out), &(j), IndexOrigin::FromEitherEnd)?)) > (&(key))) {
                {
                    let __compylr_value = py_subscript(&(out), &(j), IndexOrigin::FromEitherEnd)?;
                    let __compylr_index = PyAdd::py_add(&(j), &(1i64))?;
                    PySetItem::py_set(&mut (out), &__compylr_index, __compylr_value)?;
                }
                j = PyNum::py_sub(&(j), &(1i64))?;
            } else {
                break;
            }
        }
        {
            let __compylr_value = key.clone();
            let __compylr_index = PyAdd::py_add(&(j), &(1i64))?;
            PySetItem::py_set(&mut (out), &__compylr_index, __compylr_value)?;
        }
        PyAddAssign::py_add_assign(&mut i, &(1i64))?;
    }
    Ok(out)
}

/// The floor of the square root of `n`, by Newton's method. -1 for a negative `n`.
///
///     Integer throughout rather than a float square root and a truncation: the float version is
///     wrong for large `n`, because a 64-bit float cannot represent every integer this can.
///     
pub fn integer_sqrt(n: i64) -> Result<i64, RuntimeError> {
    if ((&(n)) < (&(0i64))) {
        return Ok(-1i64);
    }
    if ((&(n)) < (&(2i64))) {
        return Ok(n);
    }
    let mut x: i64 = n;
    let mut y: i64 = PyNum::div_floor(&(PyAdd::py_add(&(x), &(1i64))?), &(2i64))?;
    while ((&(y)) < (&(x))) {
        x = y;
        y = PyNum::div_floor(
            &(PyAdd::py_add(&(x), &(PyNum::div_floor(&(n), &(x))?))?),
            &(2i64),
        )?;
    }
    Ok(x)
}

/// Whether `xs` is in non-descending order.
///
///     The oracle every sort here is checked against, expressed in the subset so the check itself
///     is compiled too.
///     
pub fn is_sorted(xs: Vec<i64>) -> Result<bool, RuntimeError> {
    let mut i: i64 = 1i64;
    while ((&(i)) < (&(PyLen::py_len(&(xs), TextUnits::CodePoints)))) {
        if ((&(py_subscript(
            &(xs),
            &(PyNum::py_sub(&(i), &(1i64))?),
            IndexOrigin::FromEitherEnd,
        )?)) > (&(py_subscript(&(xs), &(i), IndexOrigin::FromEitherEnd)?)))
        {
            return Ok(false);
        }
        PyAddAssign::py_add_assign(&mut i, &(1i64))?;
    }
    Ok(true)
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
        PyAddAssign::py_add_assign(&mut candidate, &(1i64))?;
    }
    Ok(found)
}

/// `separator.join(words)`, written out.
///
///     There are no string methods, so this is the loop `join` would have hidden. It is also
///     quadratic — each `+` builds a new string — which `str.join` is not. Compiling something is
///     not the same as making it fast, and this is the clearest small example of that in the demo.
///     
pub fn joined(words: Vec<String>, separator: String) -> Result<String, RuntimeError> {
    let mut out: String = String::from("");
    let mut first: bool = true;
    {
        let __compylr_iter = &words;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let word: &String = __compylr_item;
            if first {
                PyAddAssign::py_add_assign(&mut out, &(word))?;
                first = false;
            } else {
                PyAddAssign::py_add_assign(&mut out, &(separator))?;
                PyAddAssign::py_add_assign(&mut out, &(word))?;
            }
        }
    }
    Ok(out)
}

/// The greatest total value that fits in `capacity`, taking each item at most once.
///
///     The classic 0/1 knapsack. `weights` and `values` are parallel lists because the subset has no
///     record type and a `list[tuple[int, int]]` would need a tuple read per access — this reads
///     better and is the shape the interpreted reference uses too.
///     
pub fn knapsack(weights: Vec<i64>, values: Vec<i64>, capacity: i64) -> Result<i64, RuntimeError> {
    let items: i64 = PyLen::py_len(&(weights), TextUnits::CodePoints);
    if ((&(items)) > (&(PyLen::py_len(&(values), TextUnits::CodePoints)))) {
        return Ok(0i64);
    }
    let mut table: Vec<Vec<i64>> = table_of_zeros(
        PyAdd::py_add(&(items), &(1i64))?,
        PyAdd::py_add(&(capacity), &(1i64))?,
    )?;
    {
        let __compylr_stop: i64 = PyAdd::py_add(&(items), &(1i64))?;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 1i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let i: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_stop: i64 = PyAdd::py_add(&(capacity), &(1i64))?;
                let __compylr_step: i64 = 1i64;
                if __compylr_step == 0 {
                    return Err(RuntimeError::ZeroStep);
                }
                let mut __compylr_cursor: i64 = 0i64;
                while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
                    || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
                {
                    let room: i64 = __compylr_cursor;
                    __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
                    {
                        let __compylr_value = py_subscript(
                            &(*py_borrow(
                                &(table),
                                &(PyNum::py_sub(&(i), &(1i64))?),
                                IndexOrigin::FromEitherEnd,
                            )?),
                            &(room),
                            IndexOrigin::FromEitherEnd,
                        )?;
                        let __compylr_index = room.clone();
                        PySetItem::py_set(
                            &mut (*py_place(&mut (table), &(i), IndexOrigin::FromEitherEnd)?),
                            &__compylr_index,
                            __compylr_value,
                        )?;
                    }
                    if ((&(py_subscript(
                        &(weights),
                        &(PyNum::py_sub(&(i), &(1i64))?),
                        IndexOrigin::FromEitherEnd,
                    )?)) > (&(room)))
                    {
                        continue;
                    }
                    let taken: i64 = PyAdd::py_add(
                        &(py_subscript(
                            &(*py_borrow(
                                &(table),
                                &(PyNum::py_sub(&(i), &(1i64))?),
                                IndexOrigin::FromEitherEnd,
                            )?),
                            &(PyNum::py_sub(
                                &(room),
                                &(py_subscript(
                                    &(weights),
                                    &(PyNum::py_sub(&(i), &(1i64))?),
                                    IndexOrigin::FromEitherEnd,
                                )?),
                            )?),
                            IndexOrigin::FromEitherEnd,
                        )?),
                        &(py_subscript(
                            &(values),
                            &(PyNum::py_sub(&(i), &(1i64))?),
                            IndexOrigin::FromEitherEnd,
                        )?),
                    )?;
                    if ((&(taken))
                        > (&(py_subscript(
                            &(*py_borrow(&(table), &(i), IndexOrigin::FromEitherEnd)?),
                            &(room),
                            IndexOrigin::FromEitherEnd,
                        )?)))
                    {
                        {
                            let __compylr_value = taken.clone();
                            let __compylr_index = room.clone();
                            PySetItem::py_set(
                                &mut (*py_place(&mut (table), &(i), IndexOrigin::FromEitherEnd)?),
                                &__compylr_index,
                                __compylr_value,
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(py_subscript(
        &(*py_borrow(&(table), &(items), IndexOrigin::FromEitherEnd)?),
        &(capacity),
        IndexOrigin::FromEitherEnd,
    )?)
}

/// The larger of two integers — `max` is not in the subset.
pub fn larger(a: i64, b: i64) -> Result<i64, RuntimeError> {
    if ((&(a)) > (&(b))) {
        return Ok(a);
    }
    Ok(b)
}

/// The least common multiple. Zero when either argument is zero, as `math.lcm` gives.
pub fn lcm(a: i64, b: i64) -> Result<i64, RuntimeError> {
    if ((&(a)) == (&(0i64))) {
        return Ok(0i64);
    }
    if ((&(b)) == (&(0i64))) {
        return Ok(0i64);
    }
    let mut product: i64 = PyNum::py_mul(&(a), &(b))?;
    if ((&(product)) < (&(0i64))) {
        product = PyNum::py_neg(&(product))?;
    }
    Ok(PyNum::div_floor(&(product), &(gcd(a, b)?))?)
}

/// The longest word, the earliest one when several tie. `""` when there are none.
pub fn longest(words: Vec<String>) -> Result<String, RuntimeError> {
    let mut best: String = String::from("");
    let mut best_length: i64 = -1i64;
    {
        let __compylr_iter = &words;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let word: &String = __compylr_item;
            if ((&(PyLen::py_len(&(word), TextUnits::CodePoints))) > (&(best_length))) {
                best = word.clone();
                best_length = PyLen::py_len(&(word), TextUnits::CodePoints);
            }
        }
    }
    Ok(best)
}

/// The length of the longest subsequence common to both lists.
///
///     Length rather than the subsequence itself: reconstructing it walks the table backwards and
///     would double the code without adding a construct the demo does not already show.
///     
pub fn longest_common_subsequence(left: Vec<i64>, right: Vec<i64>) -> Result<i64, RuntimeError> {
    let rows: i64 = PyLen::py_len(&(left), TextUnits::CodePoints);
    let columns: i64 = PyLen::py_len(&(right), TextUnits::CodePoints);
    let mut table: Vec<Vec<i64>> = table_of_zeros(
        PyAdd::py_add(&(rows), &(1i64))?,
        PyAdd::py_add(&(columns), &(1i64))?,
    )?;
    {
        let __compylr_stop: i64 = PyAdd::py_add(&(rows), &(1i64))?;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 1i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let i: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_stop: i64 = PyAdd::py_add(&(columns), &(1i64))?;
                let __compylr_step: i64 = 1i64;
                if __compylr_step == 0 {
                    return Err(RuntimeError::ZeroStep);
                }
                let mut __compylr_cursor: i64 = 1i64;
                while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
                    || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
                {
                    let j: i64 = __compylr_cursor;
                    __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
                    if ((&(py_subscript(
                        &(left),
                        &(PyNum::py_sub(&(i), &(1i64))?),
                        IndexOrigin::FromEitherEnd,
                    )?)) == (&(py_subscript(
                        &(right),
                        &(PyNum::py_sub(&(j), &(1i64))?),
                        IndexOrigin::FromEitherEnd,
                    )?))) {
                        {
                            let __compylr_value = PyAdd::py_add(
                                &(py_subscript(
                                    &(*py_borrow(
                                        &(table),
                                        &(PyNum::py_sub(&(i), &(1i64))?),
                                        IndexOrigin::FromEitherEnd,
                                    )?),
                                    &(PyNum::py_sub(&(j), &(1i64))?),
                                    IndexOrigin::FromEitherEnd,
                                )?),
                                &(1i64),
                            )?;
                            let __compylr_index = j.clone();
                            PySetItem::py_set(
                                &mut (*py_place(&mut (table), &(i), IndexOrigin::FromEitherEnd)?),
                                &__compylr_index,
                                __compylr_value,
                            )?;
                        }
                    } else {
                        {
                            let __compylr_value = larger(
                                py_subscript(
                                    &(*py_borrow(
                                        &(table),
                                        &(PyNum::py_sub(&(i), &(1i64))?),
                                        IndexOrigin::FromEitherEnd,
                                    )?),
                                    &(j),
                                    IndexOrigin::FromEitherEnd,
                                )?,
                                py_subscript(
                                    &(*py_borrow(&(table), &(i), IndexOrigin::FromEitherEnd)?),
                                    &(PyNum::py_sub(&(j), &(1i64))?),
                                    IndexOrigin::FromEitherEnd,
                                )?,
                            )?;
                            let __compylr_index = j.clone();
                            PySetItem::py_set(
                                &mut (*py_place(&mut (table), &(i), IndexOrigin::FromEitherEnd)?),
                                &__compylr_index,
                                __compylr_value,
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(py_subscript(
        &(*py_borrow(&(table), &(rows), IndexOrigin::FromEitherEnd)?),
        &(columns),
        IndexOrigin::FromEitherEnd,
    )?)
}

/// The arithmetic mean. Zero for an empty list.
///
///     Zero rather than an exception, for the reason every edge in this demo returns a sentinel:
///     the compiled subset has no exceptions of its own, so the alternative is not a better error
///     but no answer at all.
///     
pub fn mean(xs: Vec<f64>) -> Result<f64, RuntimeError> {
    if ((&(PyLen::py_len(&(xs), TextUnits::CodePoints))) == (&(0i64))) {
        return Ok(0.0f64);
    }
    let mut total: f64 = 0.0f64;
    {
        let __compylr_iter = &xs;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let x: f64 = __compylr_item;
            PyAddAssign::py_add_assign(&mut total, &(x))?;
        }
    }
    Ok(div_exact(
        &(total),
        &((PyLen::py_len(&(xs), TextUnits::CodePoints)) as f64),
    )?)
}

/// The median of an already-ascending list. Zero for an empty list.
///
///     Takes its input sorted rather than sorting it: `sorting.py` sorts integers, and a second
///     sort for floats would be the same algorithm twice. The precondition is the honest trade.
///     
pub fn median_of_sorted(xs: Vec<f64>) -> Result<f64, RuntimeError> {
    let count: i64 = PyLen::py_len(&(xs), TextUnits::CodePoints);
    if ((&(count)) == (&(0i64))) {
        return Ok(0.0f64);
    }
    if ((&(PyNum::rem_floor(&(count), &(2i64))?)) == (&(1i64))) {
        return Ok(py_subscript(
            &(xs),
            &(PyNum::div_floor(&(count), &(2i64))?),
            IndexOrigin::FromEitherEnd,
        )?);
    }
    Ok(div_exact(
        &(PyAdd::py_add(
            &(py_subscript(
                &(xs),
                &(PyNum::py_sub(&(PyNum::div_floor(&(count), &(2i64))?), &(1i64))?),
                IndexOrigin::FromEitherEnd,
            )?),
            &(py_subscript(
                &(xs),
                &(PyNum::div_floor(&(count), &(2i64))?),
                IndexOrigin::FromEitherEnd,
            )?),
        )?),
        &(2.0f64),
    )?)
}

/// Two ascending lists interleaved into one.
///
///     Stable: the `<=` is what keeps equal elements in the order they arrived, and turning it into
///     `<` would silently make merge sort unstable.
///     
pub fn merge(left: Vec<i64>, right: Vec<i64>) -> Result<Vec<i64>, RuntimeError> {
    let mut out: Vec<i64> = vec![];
    let mut i: i64 = 0i64;
    let mut j: i64 = 0i64;
    while ((&(i)) < (&(PyLen::py_len(&(left), TextUnits::CodePoints)))) {
        if ((&(j)) >= (&(PyLen::py_len(&(right), TextUnits::CodePoints)))) {
            break;
        }
        if ((&(py_subscript(&(left), &(i), IndexOrigin::FromEitherEnd)?))
            <= (&(py_subscript(&(right), &(j), IndexOrigin::FromEitherEnd)?)))
        {
            {
                let __compylr_value = py_subscript(&(left), &(i), IndexOrigin::FromEitherEnd)?;
                (out).push(__compylr_value);
            }
            PyAddAssign::py_add_assign(&mut i, &(1i64))?;
        } else {
            {
                let __compylr_value = py_subscript(&(right), &(j), IndexOrigin::FromEitherEnd)?;
                (out).push(__compylr_value);
            }
            PyAddAssign::py_add_assign(&mut j, &(1i64))?;
        }
    }
    while ((&(i)) < (&(PyLen::py_len(&(left), TextUnits::CodePoints)))) {
        {
            let __compylr_value = py_subscript(&(left), &(i), IndexOrigin::FromEitherEnd)?;
            (out).push(__compylr_value);
        }
        PyAddAssign::py_add_assign(&mut i, &(1i64))?;
    }
    while ((&(j)) < (&(PyLen::py_len(&(right), TextUnits::CodePoints)))) {
        {
            let __compylr_value = py_subscript(&(right), &(j), IndexOrigin::FromEitherEnd)?;
            (out).push(__compylr_value);
        }
        PyAddAssign::py_add_assign(&mut j, &(1i64))?;
    }
    Ok(out)
}

/// `xs` in ascending order, by merge sort. O(n log n), and stable.
///
///     The halves are built by a loop rather than by `xs[:mid]`, because slicing is not in the
///     subset. Worth knowing what that costs: each recursive call takes its argument **by value**,
///     so this copies at every level. So does the interpreted version, which builds two new lists
///     per call — the difference is that here it is a `memcpy` of a `Vec<i64>` rather than a list
///     of boxed integers.
///     
pub fn merge_sort(xs: Vec<i64>) -> Result<Vec<i64>, RuntimeError> {
    if ((&(PyLen::py_len(&(xs), TextUnits::CodePoints))) <= (&(1i64))) {
        return Ok(copy_of(xs.clone())?);
    }
    let middle: i64 = PyNum::div_floor(&(PyLen::py_len(&(xs), TextUnits::CodePoints)), &(2i64))?;
    let mut left: Vec<i64> = vec![];
    let mut right: Vec<i64> = vec![];
    let mut index: i64 = 0i64;
    {
        let __compylr_iter = &xs;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let x: i64 = __compylr_item;
            if ((&(index)) < (&(middle))) {
                {
                    let __compylr_value = x.clone();
                    (left).push(__compylr_value);
                }
            } else {
                {
                    let __compylr_value = x.clone();
                    (right).push(__compylr_value);
                }
            }
            PyAddAssign::py_add_assign(&mut index, &(1i64))?;
        }
    }
    Ok(merge(
        merge_sort(left.clone())?,
        merge_sort(right.clone())?,
    )?)
}

/// The needles that do not appear in `haystack`, in the order given.
///
///     `not in` is the only negation the subset has — there is no `not` operator — and it is not a
///     second form of membership: it lowers to the negation of one, so nothing downstream has to
///     remember to honour a flag.
///     
pub fn missing(haystack: String, needles: Vec<String>) -> Result<Vec<String>, RuntimeError> {
    let mut out: Vec<String> = vec![];
    {
        let __compylr_iter = &needles;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let needle: &String = __compylr_item;
            if !(PyContains::py_contains(&(haystack), &(needle))) {
                {
                    let __compylr_value = needle.clone();
                    (out).push(__compylr_value);
                }
            }
        }
    }
    Ok(out)
}

/// The most frequent word, ties broken by taking the alphabetically first. `""` when empty.
///
///     The tie-break is not decoration. Iterating a mapping yields its keys in **no guaranteed
///     order**, and the order varies between runs — so "whichever tied word came first" would be a
///     different answer on different runs of the same program. Any function that iterates a mapping
///     and returns one element needs a rule like this one, or it is not a function.
///     
pub fn most_common(words: Vec<String>) -> Result<String, RuntimeError> {
    let counts: FastMap<String, i64> = word_count(words.clone())?;
    let mut best: String = String::from("");
    let mut best_count: i64 = 0i64;
    {
        let __compylr_iter = &counts;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let word: String = __compylr_item;
            if ((&(py_subscript(&(counts), &(word), IndexOrigin::FromEitherEnd)?))
                > (&(best_count)))
            {
                best = word.clone();
                best_count = py_subscript(&(counts), &(word), IndexOrigin::FromEitherEnd)?;
            } else {
                if ((&(py_subscript(&(counts), &(word), IndexOrigin::FromEitherEnd)?))
                    == (&(best_count)))
                {
                    if ((&(word)) < (&(best))) {
                        best = word.clone();
                    }
                }
            }
        }
    }
    Ok(best)
}

/// The matrix product. Empty when the shapes do not line up.
///
///     Empty rather than an exception, as everywhere else in this demo — and checked rather than
///     assumed, because indexing past the end of a row is a panic in the generated code, which
///     reaches Python as an exception but not one that says anything useful about matrices.
///     
pub fn multiply(left: Vec<Vec<i64>>, right: Vec<Vec<i64>>) -> Result<Vec<Vec<i64>>, RuntimeError> {
    let rows: i64 = PyLen::py_len(&(left), TextUnits::CodePoints);
    if ((&(rows)) == (&(0i64))) {
        return Ok(vec![]);
    }
    let inner: i64 = PyLen::py_len(
        &(*py_borrow(&(left), &(0i64), IndexOrigin::FromEitherEnd)?),
        TextUnits::CodePoints,
    );
    if ((&(PyLen::py_len(&(right), TextUnits::CodePoints))) != (&(inner))) {
        return Ok(vec![]);
    }
    if ((&(inner)) == (&(0i64))) {
        return Ok(vec![]);
    }
    let columns: i64 = PyLen::py_len(
        &(*py_borrow(&(right), &(0i64), IndexOrigin::FromEitherEnd)?),
        TextUnits::CodePoints,
    );
    let mut out: Vec<Vec<i64>> = table_of_zeros(rows, columns)?;
    {
        let __compylr_stop: i64 = rows;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let i: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_stop: i64 = columns;
                let __compylr_step: i64 = 1i64;
                if __compylr_step == 0 {
                    return Err(RuntimeError::ZeroStep);
                }
                let mut __compylr_cursor: i64 = 0i64;
                while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
                    || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
                {
                    let j: i64 = __compylr_cursor;
                    __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
                    let mut total: i64 = 0i64;
                    {
                        let __compylr_stop: i64 = inner;
                        let __compylr_step: i64 = 1i64;
                        if __compylr_step == 0 {
                            return Err(RuntimeError::ZeroStep);
                        }
                        let mut __compylr_cursor: i64 = 0i64;
                        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
                            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
                        {
                            let k: i64 = __compylr_cursor;
                            __compylr_cursor =
                                PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
                            PyAddAssign::py_add_assign(
                                &mut total,
                                &(PyNum::py_mul(
                                    &(py_subscript(
                                        &(*py_borrow(&(left), &(i), IndexOrigin::FromEitherEnd)?),
                                        &(k),
                                        IndexOrigin::FromEitherEnd,
                                    )?),
                                    &(py_subscript(
                                        &(*py_borrow(&(right), &(k), IndexOrigin::FromEitherEnd)?),
                                        &(j),
                                        IndexOrigin::FromEitherEnd,
                                    )?),
                                )?),
                            )?;
                        }
                    }
                    {
                        let __compylr_value = total.clone();
                        let __compylr_index = j.clone();
                        PySetItem::py_set(
                            &mut (*py_place(&mut (out), &(i), IndexOrigin::FromEitherEnd)?),
                            &__compylr_index,
                            __compylr_value,
                        )?;
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Every node the graph mentions — keys and neighbours alike — in ascending order.
///
///     Note the annotation on `ordered`. `merge_sort` lives in another module, so at the moment this
///     function is validated its signature is not visible and the binding's type cannot be inferred.
///     Rejecting the call instead would make whether your code compiles depend on which function you
///     happened to decorate first; requiring the annotation does not. The call is still checked —
///     once every source is assembled into one unit.
///     
pub fn node_list(graph: FastMap<i64, Vec<i64>>) -> Result<Vec<i64>, RuntimeError> {
    let mut seen: FastMap<i64, i64> = FastMap::from_iter([]);
    let mut raw: Vec<i64> = vec![];
    {
        let __compylr_iter = &graph;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let node: i64 = __compylr_item;
            if !(PyContains::py_contains(&(seen), &(node))) {
                {
                    let __compylr_value = 1i64;
                    let __compylr_index = node.clone();
                    PySetItem::py_set(&mut (seen), &__compylr_index, __compylr_value)?;
                }
                {
                    let __compylr_value = node.clone();
                    (raw).push(__compylr_value);
                }
            }
            {
                let __compylr_iter = &(*py_borrow(&(graph), &(node), IndexOrigin::FromEitherEnd)?);
                for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                    let neighbour: i64 = __compylr_item;
                    if PyContains::py_contains(&(seen), &(neighbour)) {
                        continue;
                    }
                    {
                        let __compylr_value = 1i64;
                        let __compylr_index = neighbour.clone();
                        PySetItem::py_set(&mut (seen), &__compylr_index, __compylr_value)?;
                    }
                    {
                        let __compylr_value = neighbour.clone();
                        (raw).push(__compylr_value);
                    }
                }
            }
        }
    }
    let ordered: Vec<i64> = merge_sort(raw.clone())?;
    Ok(ordered)
}

/// `xs` rescaled so its smallest value is 0.0 and its largest is 1.0.
///
///     A constant input has no span to divide by, and maps to all zeros rather than dividing by
///     zero. Float division by zero is the one arithmetic hazard here that does **not** raise —
///     IEEE-754 says it is an infinity — so guarding is the only thing that keeps the answer finite.
///     
pub fn normalize(xs: Vec<f64>) -> Result<Vec<f64>, RuntimeError> {
    let span: (f64, f64) = extremes(xs.clone())?;
    let lowest: f64 = (span).0.clone();
    let highest: f64 = (span).1.clone();
    let mut out: Vec<f64> = vec![];
    if ((&(PyNum::py_sub(&(highest), &(lowest))?)) == (&(0.0f64))) {
        {
            let __compylr_iter = &xs;
            for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                let _x: f64 = __compylr_item;
                {
                    let __compylr_value = 0.0f64;
                    (out).push(__compylr_value);
                }
            }
        }
        return Ok(out.clone());
    }
    {
        let __compylr_iter = &xs;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let x: f64 = __compylr_item;
            {
                let __compylr_value = div_exact(
                    &(PyNum::py_sub(&(x), &(lowest))?),
                    &(PyNum::py_sub(&(highest), &(lowest))?),
                )?;
                (out).push(__compylr_value);
            }
        }
    }
    Ok(out)
}

/// How many of `needles` appear anywhere in `haystack`.
///
///     `in` over a string tests for a **substring**, matching Python — and matching Go, C++, and
///     TypeScript too, which is why it is one of the three container behaviours the IR deliberately
///     does *not* make configurable.
///     
pub fn occurrences(haystack: String, needles: Vec<String>) -> Result<i64, RuntimeError> {
    let mut total: i64 = 0i64;
    {
        let __compylr_iter = &needles;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let needle: &String = __compylr_item;
            if PyContains::py_contains(&(haystack), &(needle)) {
                PyAddAssign::py_add_assign(&mut total, &(1i64))?;
            }
        }
    }
    Ok(total)
}

/// `base` raised to `exponent`, by squaring. Zero for a negative exponent.
///
///     The subset has no `**`. Note the guard before the squaring: the obvious loop squares `base`
///     once more than it needs, and for an exponent near the top of the range that overflows even
///     when the answer does not. Overflow is reported here rather than wrapping, so the obvious
///     version fails loudly on inputs this one answers.
///     
pub fn power(base: i64, exponent: i64) -> Result<i64, RuntimeError> {
    if ((&(exponent)) < (&(0i64))) {
        return Ok(0i64);
    }
    let mut result: i64 = 1i64;
    let mut factor: i64 = base;
    let mut remaining: i64 = exponent;
    while ((&(remaining)) > (&(0i64))) {
        if ((&(PyNum::rem_floor(&(remaining), &(2i64))?)) == (&(1i64))) {
            result = PyNum::py_mul(&(result), &(factor))?;
        }
        remaining = PyNum::div_floor(&(remaining), &(2i64))?;
        if ((&(remaining)) > (&(0i64))) {
            factor = PyNum::py_mul(&(factor), &(factor))?;
        }
    }
    Ok(result)
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
        PyAddAssign::py_add_assign(&mut d, &(1i64))?;
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
        PyAddAssign::py_add_assign(&mut candidate, &(1i64))?;
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

/// `a % b`, taking the sign of the **divisor** as Python does.
///
///     `-7 % 2` is `1` here and `-1` in Rust. The companion to `floor_divide`, and the two are
///     consistent: `(a // b) * b + a % b == a` under either convention, but only one of them agrees
///     with the interpreted original.
///     
pub fn remainder(a: i64, b: i64) -> Result<i64, RuntimeError> {
    Ok(PyNum::rem_floor(&(a), &(b))?)
}

/// The total of each row.
///
///     The `for row in matrix` loop reads each row once. A `for` **snapshots what it iterates**, so
///     rebinding `matrix` in the body could not change what is walked — which is what Python's `for`
///     does too, and is why the emitted loop clones rather than holding a borrow across the body.
///     
pub fn row_sums(matrix: Vec<Vec<i64>>) -> Result<Vec<i64>, RuntimeError> {
    let mut out: Vec<i64> = vec![];
    {
        let __compylr_iter = &matrix;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let row: &Vec<i64> = __compylr_item;
            let mut total: i64 = 0i64;
            {
                let __compylr_iter = &row;
                for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                    let value: i64 = __compylr_item;
                    PyAddAssign::py_add_assign(&mut total, &(value))?;
                }
            }
            {
                let __compylr_value = total.clone();
                (out).push(__compylr_value);
            }
        }
    }
    Ok(out)
}

/// Every element multiplied by `factor`.
///
///     Builds a fresh matrix rather than writing into the argument. It could not do otherwise: a
///     collection parameter crosses the boundary by value, so a mutation here would be invisible to
///     the caller — and compylr rejects mutating a parameter for exactly that reason rather than
///     compiling a program whose two versions disagree.
///     
pub fn scale(matrix: Vec<Vec<i64>>, factor: i64) -> Result<Vec<Vec<i64>>, RuntimeError> {
    let mut out: Vec<Vec<i64>> = vec![];
    {
        let __compylr_iter = &matrix;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let row: &Vec<i64> = __compylr_item;
            let mut scaled: Vec<i64> = vec![];
            {
                let __compylr_iter = &row;
                for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                    let value: i64 = __compylr_item;
                    {
                        let __compylr_value = PyNum::py_mul(&(value), &(factor))?;
                        (scaled).push(__compylr_value);
                    }
                }
            }
            {
                let __compylr_value = scaled.clone();
                (out).push(__compylr_value);
            }
        }
    }
    Ok(out)
}

/// `xs` in ascending order, by repeatedly selecting the smallest remaining element.
///
///     Included beside insertion sort because it is where the missing swap shows: exchanging two
///     elements takes a temporary, and forgetting it is a bug the compiler cannot catch.
///     
pub fn selection_sort(xs: Vec<i64>) -> Result<Vec<i64>, RuntimeError> {
    let mut out: Vec<i64> = copy_of(xs.clone())?;
    let mut i: i64 = 0i64;
    while ((&(i)) < (&(PyLen::py_len(&(out), TextUnits::CodePoints)))) {
        let mut smallest: i64 = i;
        let mut j: i64 = PyAdd::py_add(&(i), &(1i64))?;
        while ((&(j)) < (&(PyLen::py_len(&(out), TextUnits::CodePoints)))) {
            if ((&(py_subscript(&(out), &(j), IndexOrigin::FromEitherEnd)?))
                < (&(py_subscript(&(out), &(smallest), IndexOrigin::FromEitherEnd)?)))
            {
                smallest = j;
            }
            PyAddAssign::py_add_assign(&mut j, &(1i64))?;
        }
        let held: i64 = py_subscript(&(out), &(i), IndexOrigin::FromEitherEnd)?;
        {
            let __compylr_value = py_subscript(&(out), &(smallest), IndexOrigin::FromEitherEnd)?;
            let __compylr_index = i.clone();
            PySetItem::py_set(&mut (out), &__compylr_index, __compylr_value)?;
        }
        {
            let __compylr_value = held.clone();
            let __compylr_index = smallest.clone();
            PySetItem::py_set(&mut (out), &__compylr_index, __compylr_value)?;
        }
        PyAddAssign::py_add_assign(&mut i, &(1i64))?;
    }
    Ok(out)
}

/// Every prime below `limit`, by the sieve of Eratosthenes.
///
///     Two `continue`s, both load-bearing, and the reason this is in the demo: `continue` inside a
///     `for` over a `range` used to skip the loop's cursor increment and hang. It was found by the
///     compiler's own conformance corpus rather than by a test written in Python, which is why that
///     corpus is checked over `(statement, position)` pairs instead of statements alone.
///     
pub fn sieve(limit: i64) -> Result<Vec<i64>, RuntimeError> {
    if ((&(limit)) < (&(3i64))) {
        return Ok(vec![]);
    }
    let mut composite: Vec<bool> = vec![];
    {
        let __compylr_stop: i64 = limit;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let _slot: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_value = false;
                (composite).push(__compylr_value);
            }
        }
    }
    let mut candidate: i64 = 2i64;
    while ((&(PyNum::py_mul(&(candidate), &(candidate))?)) < (&(limit))) {
        if py_subscript(&(composite), &(candidate), IndexOrigin::FromEitherEnd)? {
            PyAddAssign::py_add_assign(&mut candidate, &(1i64))?;
            continue;
        }
        let mut multiple: i64 = PyNum::py_mul(&(candidate), &(candidate))?;
        while ((&(multiple)) < (&(limit))) {
            {
                let __compylr_value = true;
                let __compylr_index = multiple.clone();
                PySetItem::py_set(&mut (composite), &__compylr_index, __compylr_value)?;
            }
            PyAddAssign::py_add_assign(&mut multiple, &(candidate))?;
        }
        PyAddAssign::py_add_assign(&mut candidate, &(1i64))?;
    }
    let mut primes: Vec<i64> = vec![];
    {
        let __compylr_stop: i64 = limit;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 2i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let n: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            if py_subscript(&(composite), &(n), IndexOrigin::FromEitherEnd)? {
                continue;
            }
            {
                let __compylr_value = n.clone();
                (primes).push(__compylr_value);
            }
        }
    }
    Ok(primes)
}

/// The smaller of two integers — `min` is not in the subset.
pub fn smaller(a: i64, b: i64) -> Result<i64, RuntimeError> {
    if ((&(a)) < (&(b))) {
        return Ok(a);
    }
    Ok(b)
}

/// The square root of `value`, by Newton's method. Zero for a negative input.
///
///     Forty iterations rather than a convergence test: the loop count is then a constant, which
///     makes this a fair thing to benchmark, and forty is far past the point where a 64-bit float
///     stops changing.
///     
pub fn square_root(value: f64) -> Result<f64, RuntimeError> {
    if ((&(value)) <= (&(0.0f64))) {
        return Ok(0.0f64);
    }
    let mut guess: f64 = value;
    let mut step: i64 = 0i64;
    while ((&(step)) < (&(40i64))) {
        guess = div_exact(
            &(PyAdd::py_add(&(guess), &(div_exact(&(value), &(guess))?))?),
            &(2.0f64),
        )?;
        PyAddAssign::py_add_assign(&mut step, &(1i64))?;
    }
    Ok(guess)
}

/// The population standard deviation.
pub fn standard_deviation(xs: Vec<f64>) -> Result<f64, RuntimeError> {
    Ok(square_root(variance(xs.clone())?)?)
}

/// A `rows` by `columns` table of zeros.
///
///     Written once and called by everything below. Each row is appended as a freshly built list,
///     which is not a detail: a version that built one row and appended it `rows` times would give
///     every row the same identity in Python and independent rows here, so the two languages would
///     disagree about what writing to one of them does.
///     
pub fn table_of_zeros(rows: i64, columns: i64) -> Result<Vec<Vec<i64>>, RuntimeError> {
    let mut table: Vec<Vec<i64>> = vec![];
    {
        let __compylr_stop: i64 = rows;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let _row: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            let mut line: Vec<i64> = vec![];
            {
                let __compylr_stop: i64 = columns;
                let __compylr_step: i64 = 1i64;
                if __compylr_step == 0 {
                    return Err(RuntimeError::ZeroStep);
                }
                let mut __compylr_cursor: i64 = 0i64;
                while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
                    || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
                {
                    let _column: i64 = __compylr_cursor;
                    __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
                    {
                        let __compylr_value = 0i64;
                        (line).push(__compylr_value);
                    }
                }
            }
            {
                let __compylr_value = line.clone();
                (table).push(__compylr_value);
            }
        }
    }
    Ok(table)
}

/// The digits of `n` in `base`, most significant first. Negative `n` is treated as positive.
///
///     A `base` of zero divides by zero, which is **reported** rather than being undefined: the
///     guarantee the Python frontend requires is that division by zero raises, and the Rust backend
///     preserves it. The exception surfaces on the Python side as `ZeroDivisionError`.
///     
pub fn to_base(n: i64, base: i64) -> Result<Vec<i64>, RuntimeError> {
    let mut current: i64 = n;
    if ((&(current)) < (&(0i64))) {
        current = PyNum::py_neg(&(current))?;
    }
    let mut backwards: Vec<i64> = vec![];
    if ((&(current)) == (&(0i64))) {
        {
            let __compylr_value = 0i64;
            (backwards).push(__compylr_value);
        }
    }
    while ((&(current)) > (&(0i64))) {
        let split: (i64, i64) = divide(current, base)?;
        {
            let __compylr_value = (split).1.clone();
            (backwards).push(__compylr_value);
        }
        current = (split).0.clone();
    }
    let mut digits: Vec<i64> = vec![];
    let mut index: i64 = PyNum::py_sub(
        &(PyLen::py_len(&(backwards), TextUnits::CodePoints)),
        &(1i64),
    )?;
    while ((&(index)) >= (&(0i64))) {
        {
            let __compylr_value = py_subscript(&(backwards), &(index), IndexOrigin::FromEitherEnd)?;
            (digits).push(__compylr_value);
        }
        index = PyNum::py_sub(&(index), &(1i64))?;
    }
    Ok(digits)
}

/// An order in which every edge points forward. Empty when the graph has a cycle.
///
///     Kahn's algorithm, always taking the **smallest** ready node rather than the first the mapping
///     offered — `node_list` is ascending, so scanning it in order does that. Without the rule this
///     would return a different valid order on different runs of the same program, and a test that
///     pinned one of them would be flaky rather than the compiler being wrong.
///
///     An empty result is ambiguous for an empty graph, which has no order to return either way.
///     `has_cycle` is the unambiguous question.
///     
pub fn topological_order(graph: FastMap<i64, Vec<i64>>) -> Result<Vec<i64>, RuntimeError> {
    let nodes: Vec<i64> = node_list(graph.clone())?;
    let mut indegree: FastMap<i64, i64> = FastMap::from_iter([]);
    {
        let __compylr_iter = &nodes;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let node: i64 = __compylr_item;
            {
                let __compylr_value = 0i64;
                let __compylr_index = node.clone();
                PySetItem::py_set(&mut (indegree), &__compylr_index, __compylr_value)?;
            }
        }
    }
    {
        let __compylr_iter = &nodes;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let node: i64 = __compylr_item;
            if !(PyContains::py_contains(&(graph), &(node))) {
                continue;
            }
            {
                let __compylr_iter = &(*py_borrow(&(graph), &(node), IndexOrigin::FromEitherEnd)?);
                for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                    let neighbour: i64 = __compylr_item;
                    {
                        let __compylr_value = PyAdd::py_add(
                            &(py_subscript(&(indegree), &(neighbour), IndexOrigin::FromEitherEnd)?),
                            &(1i64),
                        )?;
                        let __compylr_index = neighbour.clone();
                        PySetItem::py_set(&mut (indegree), &__compylr_index, __compylr_value)?;
                    }
                }
            }
        }
    }
    let mut order: Vec<i64> = vec![];
    let mut placed: FastMap<i64, i64> = FastMap::from_iter([]);
    while ((&(PyLen::py_len(&(order), TextUnits::CodePoints)))
        < (&(PyLen::py_len(&(nodes), TextUnits::CodePoints))))
    {
        let mut ready: bool = false;
        let mut chosen: i64 = 0i64;
        {
            let __compylr_iter = &nodes;
            for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                let node: i64 = __compylr_item;
                if PyContains::py_contains(&(placed), &(node)) {
                    continue;
                }
                if ((&(py_subscript(&(indegree), &(node), IndexOrigin::FromEitherEnd)?))
                    == (&(0i64)))
                {
                    chosen = node;
                    ready = true;
                    break;
                }
            }
        }
        if ready {
            {
                let __compylr_value = 1i64;
                let __compylr_index = chosen.clone();
                PySetItem::py_set(&mut (placed), &__compylr_index, __compylr_value)?;
            }
            {
                let __compylr_value = chosen.clone();
                (order).push(__compylr_value);
            }
            if PyContains::py_contains(&(graph), &(chosen)) {
                {
                    let __compylr_iter =
                        &(*py_borrow(&(graph), &(chosen), IndexOrigin::FromEitherEnd)?);
                    for __compylr_item in PyIterate::py_iter(__compylr_iter) {
                        let neighbour: i64 = __compylr_item;
                        {
                            let __compylr_value = PyNum::py_sub(
                                &(py_subscript(
                                    &(indegree),
                                    &(neighbour),
                                    IndexOrigin::FromEitherEnd,
                                )?),
                                &(1i64),
                            )?;
                            let __compylr_index = neighbour.clone();
                            PySetItem::py_set(&mut (indegree), &__compylr_index, __compylr_value)?;
                        }
                    }
                }
            }
        } else {
            return Ok(vec![]);
        }
    }
    Ok(order)
}

/// The combined length of every word, in **code points**.
///
///     Not bytes. `len("é")` is 1 here and would be 2 under Go's reading of the same operation; the
///     IR records which, so the answer does not depend on which backend ran.
///     
pub fn total_length(words: Vec<String>) -> Result<i64, RuntimeError> {
    let mut total: i64 = 0i64;
    {
        let __compylr_iter = &words;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let word: &String = __compylr_item;
            PyAddAssign::py_add_assign(
                &mut total,
                &(PyLen::py_len(&(word), TextUnits::CodePoints)),
            )?;
        }
    }
    Ok(total)
}

/// The sum along the leading diagonal. Zero for an empty matrix.
pub fn trace(matrix: Vec<Vec<i64>>) -> Result<i64, RuntimeError> {
    let rows: i64 = PyLen::py_len(&(matrix), TextUnits::CodePoints);
    if ((&(rows)) == (&(0i64))) {
        return Ok(0i64);
    }
    let columns: i64 = PyLen::py_len(
        &(*py_borrow(&(matrix), &(0i64), IndexOrigin::FromEitherEnd)?),
        TextUnits::CodePoints,
    );
    let mut total: i64 = 0i64;
    let mut limit: i64 = rows;
    if ((&(columns)) < (&(limit))) {
        limit = columns;
    }
    {
        let __compylr_stop: i64 = limit;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let i: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            PyAddAssign::py_add_assign(
                &mut total,
                &(py_subscript(
                    &(*py_borrow(&(matrix), &(i), IndexOrigin::FromEitherEnd)?),
                    &(i),
                    IndexOrigin::FromEitherEnd,
                )?),
            )?;
        }
    }
    Ok(total)
}

/// `matrix` with rows and columns exchanged. Empty for an empty matrix.
pub fn transpose(matrix: Vec<Vec<i64>>) -> Result<Vec<Vec<i64>>, RuntimeError> {
    let rows: i64 = PyLen::py_len(&(matrix), TextUnits::CodePoints);
    if ((&(rows)) == (&(0i64))) {
        return Ok(vec![]);
    }
    let columns: i64 = PyLen::py_len(
        &(*py_borrow(&(matrix), &(0i64), IndexOrigin::FromEitherEnd)?),
        TextUnits::CodePoints,
    );
    let mut out: Vec<Vec<i64>> = table_of_zeros(columns, rows)?;
    {
        let __compylr_stop: i64 = rows;
        let __compylr_step: i64 = 1i64;
        if __compylr_step == 0 {
            return Err(RuntimeError::ZeroStep);
        }
        let mut __compylr_cursor: i64 = 0i64;
        while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
            || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
        {
            let i: i64 = __compylr_cursor;
            __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
            {
                let __compylr_stop: i64 = columns;
                let __compylr_step: i64 = 1i64;
                if __compylr_step == 0 {
                    return Err(RuntimeError::ZeroStep);
                }
                let mut __compylr_cursor: i64 = 0i64;
                while (__compylr_step > 0 && __compylr_cursor < __compylr_stop)
                    || (__compylr_step < 0 && __compylr_cursor > __compylr_stop)
                {
                    let j: i64 = __compylr_cursor;
                    __compylr_cursor = PyAdd::py_add(&(__compylr_cursor), &(__compylr_step))?;
                    {
                        let __compylr_value = py_subscript(
                            &(*py_borrow(&(matrix), &(i), IndexOrigin::FromEitherEnd)?),
                            &(j),
                            IndexOrigin::FromEitherEnd,
                        )?;
                        let __compylr_index = i.clone();
                        PySetItem::py_set(
                            &mut (*py_place(&mut (out), &(j), IndexOrigin::FromEitherEnd)?),
                            &__compylr_index,
                            __compylr_value,
                        )?;
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Duplicates removed, **first-seen order kept**.
///
///     A list rather than a set, deliberately. A set would express the intent better and would lose
///     the order — sets and mappings both iterate in whatever order the underlying map gives. The
///     mapping here is used as a seen-set, which is what it is good for: membership, not order.
///     
pub fn unique_words(words: Vec<String>) -> Result<Vec<String>, RuntimeError> {
    let mut seen: FastMap<String, i64> = FastMap::from_iter([]);
    let mut out: Vec<String> = vec![];
    {
        let __compylr_iter = &words;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let word: &String = __compylr_item;
            if PyContains::py_contains(&(seen), &(word)) {
                continue;
            }
            {
                let __compylr_value = 1i64;
                let __compylr_index = word.clone();
                PySetItem::py_set(&mut (seen), &__compylr_index, __compylr_value)?;
            }
            {
                let __compylr_value = word.clone();
                (out).push(__compylr_value);
            }
        }
    }
    Ok(out)
}

/// The population variance — the mean of the squared deviations. Zero for an empty list.
///
///     Two passes rather than the one-pass sum-of-squares identity. The identity is faster and
///     loses catastrophic precision when the mean is large relative to the spread, which is exactly
///     the case where somebody would trust a compiled answer more than an interpreted one.
///     
pub fn variance(xs: Vec<f64>) -> Result<f64, RuntimeError> {
    if ((&(PyLen::py_len(&(xs), TextUnits::CodePoints))) == (&(0i64))) {
        return Ok(0.0f64);
    }
    let centre: f64 = mean(xs.clone())?;
    let mut total: f64 = 0.0f64;
    {
        let __compylr_iter = &xs;
        for __compylr_item in PyIterate::py_iter(__compylr_iter) {
            let x: f64 = __compylr_item;
            let deviation: f64 = PyNum::py_sub(&(x), &(centre))?;
            PyAddAssign::py_add_assign(&mut total, &(PyNum::py_mul(&(deviation), &(deviation))?))?;
        }
    }
    Ok(div_exact(
        &(total),
        &((PyLen::py_len(&(xs), TextUnits::CodePoints)) as f64),
    )?)
}

/// The five vowels, as a set.
///
///     A set literal is the only way to build one: there is no `add`, and `set()` is not a call the
///     subset resolves. A set is therefore something you receive, test against, or return whole —
///     which covers what a lookup table is for, and not much else.
///     
pub fn vowel_letters() -> Result<FastSet<String>, RuntimeError> {
    Ok(FastSet::from_iter([
        String::from("a"),
        String::from("e"),
        String::from("i"),
        String::from("o"),
        String::from("u"),
    ]))
}

/// How many times each word appears.
///
///     `word in counts` tests the mapping's keys, as Python does. Reading a key that is absent is
///     still an error — assignment is what creates one — so the `else` is not optional.
///     
pub fn word_count(words: Vec<String>) -> Result<FastMap<String, i64>, RuntimeError> {
    let mut counts: FastMap<String, i64> = FastMap::from_iter([]);
    {
        let __compylr_iter = &words;
        for __compylr_item in PyIterate::py_iter_borrowed(__compylr_iter) {
            let word: &String = __compylr_item;
            if PyContains::py_contains(&(counts), &(word)) {
                {
                    let __compylr_value = PyAdd::py_add(
                        &(py_subscript(&(counts), &(word), IndexOrigin::FromEitherEnd)?),
                        &(1i64),
                    )?;
                    let __compylr_index = word.clone();
                    PySetItem::py_set(&mut (counts), &__compylr_index, __compylr_value)?;
                }
            } else {
                {
                    let __compylr_value = 1i64;
                    let __compylr_index = word.clone();
                    PySetItem::py_set(&mut (counts), &__compylr_index, __compylr_value)?;
                }
            }
        }
    }
    Ok(counts)
}
