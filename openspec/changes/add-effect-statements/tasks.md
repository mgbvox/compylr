## 1. Effectful operations in the registry

- [ ] 1.1 Write the failing tests: an effectful operation is accepted as a statement; the same
      operation in a value position is refused; a result-producing intrinsic as a statement is
      still refused.
- [ ] 1.2 Record on each registry entry whether it produces a result or is performed for effect.
- [ ] 1.3 Add the `print` entry as effectful, accepting any number of positional arguments.
- [ ] 1.4 Confirm an effectful entry declares no result type and that a value position cannot
      resolve one.

## 2. The IR statement form

- [ ] 2.1 Write the failing tests: the form round-trips through the artifact; it is distinct from
      the existing effect statement; the rendering convention changes the fingerprint.
- [ ] 2.2 Add the effectful-intrinsic statement form to
      [`Stmt`](../../../crates/compylr-ir/src/ir.rs#L728), carrying operation, arguments, and
      rendering convention.
- [ ] 2.3 Leave [`Stmt::Effect`](../../../crates/compylr-ir/src/ir.rs#L761) and its documented
      meaning untouched; assert existing programs using it are unchanged.
- [ ] 2.4 Add the rendering convention type and default it to the source language's.
- [ ] 2.5 Advance [`ARTIFACT_VERSION`](../../../crates/compylr-ir/src/ir.rs#L58) and extend
      [`Unit::fingerprint`](../../../crates/compylr-ir/src/ir.rs#L1299) over the operation, the
      arguments, and the convention.

## 3. Lowering

- [ ] 3.1 Write the failing tests: an effectful intrinsic statement lowers; a mapping argument is
      refused with its reason; a set argument likewise; a nested unordered element likewise; an
      instance argument is refused.
- [ ] 3.2 Replace the shape-based carve-out in
      [`bare_expression_error`](../../../crates/compylr-frontend-python/src/lower.rs#L1677) with a
      registry question, so `append` and method effects keep working and every later effectful
      operation is covered without another exception.
- [ ] 3.3 Check each argument's type has a defined rendering, and refuse a mapping or set naming the
      unspecified iteration order as the reason.
- [ ] 3.4 Make the refusal name the ordered-projection workaround.
- [ ] 3.5 Confirm keyword arguments still fail through the existing rejection, with no new
      diagnostic added.
- [ ] 3.6 Take the rendering convention from the resolved behavior, never from the operation's name.

## 4. Rust runtime and emission

- [ ] 4.1 Write the failing tests: output emits a sink call and not a direct write; the emitted
      crate names no host; rendering uses the convention's renderer.
- [ ] 4.2 Add the output sink to [`runtime.rs`](../../../crates/compylr-backend-rust/src/runtime.rs),
      with a default writing to the target's own standard output.
- [ ] 4.3 Implement the source-convention renderers for integer, float, boolean, string, sequence,
      and tuple, matching CPython's text exactly — `True` not `true`, `5.0` not `5`.
- [ ] 4.4 Render a sequence into a single buffer rather than allocating per element; add a test that
      would fail on a per-element allocation.
- [ ] 4.5 Emit the output statement in [`rust.rs`](../../../crates/compylr-backend-rust/src/rust.rs)
      as a sink call with rendered arguments, the convention's separator, and its terminator.
- [ ] 4.6 Extend [`conformance.rs`](../../../crates/compylr-host-python/tests/conformance.rs) to
      cover the new statement form in all four positions — free function, method, constructor, and
      loop body.
- [ ] 4.7 Confirm emission is byte-reproducible and performs no I/O.

## 5. The bridge

- [ ] 5.1 Write the failing tests: ordering is preserved across the boundary under a pipe; host
      redirection captures compiled output; a write failure raises.
- [ ] 5.2 Install the host sink in
      [`compylr-bridge-python-rust`](../../../crates/compylr-bridge-python-rust/src/lib.rs) when the
      generated extension module is imported, before any compiled function can run.
- [ ] 5.3 Write through the host's own stream so `redirect_stdout`, `capsys`, and notebook capture
      all see the output.
- [ ] 5.4 Acquire and release the host stream per write; never hold it across a call into user code.
- [ ] 5.5 Surface a write failure as an exception rather than discarding it.
- [ ] 5.6 Confirm [`crate_boundaries.rs`](../../../crates/compylr-host-python/tests/crate_boundaries.rs)
      still passes — the backend must not have gained a host dependency.

## 6. Corpus

- [ ] 6.1 Move the proposal's worked example into
      [`accepted/`](../../../frontends/python/fixtures/accepted/) as `printing.py`, extended to
      print every renderable type, with member names unique across the whole accepted corpus.
- [ ] 6.2 Add its driver in [`drivers/`](../../../frontends/python/fixtures/drivers/), carrying no
      expected output.
- [ ] 6.3 Extend the differential harness in
      [`differential.rs`](../../../crates/compylr-host-python/tests/differential.rs) to capture and
      compare output text from both tiers, and to compare line order.
- [ ] 6.4 Add a fixture interleaving driver output with compiled output, to cover ordering.
- [ ] 6.5 Add rejected fixtures in [`rejected/`](../../../frontends/python/fixtures/rejected/):
      printing a mapping, printing a set, printing a sequence of mappings, printing an instance, and
      an effectful intrinsic in a value position.
- [ ] 6.6 Confirm `COMPYLR_DISABLE=1` and the compiled run produce identical output.

## 7. Demo coverage

- [ ] 7.1 Add an algorithm to [`demo/demo-python-rust/src/algorithms/`](../../../demo/demo-python-rust/src/algorithms/)
      that prints, so the added `Stmt` form is covered rather than the claim narrowed.
- [ ] 7.2 Confirm [`demo_coverage.rs`](../../../crates/compylr-host-python/tests/demo_coverage.rs)
      passes and `ir_coverage.py` sees the new statement form.

## 8. Documentation and checks

- [ ] 8.1 Regenerate the README subset matrix with
      [`update_subset.py`](../../../scripts/update_subset.py); confirm `--check` passes.
- [ ] 8.2 Update README prose: output, the rendering rule, and why unordered containers are refused.
- [ ] 8.3 Update [`CLAUDE.md`](../../../CLAUDE.md): the new statement form, the sink and why it is
      not a direct write, the rendering convention, and the artifact version.
- [ ] 8.4 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and
      `cargo test --workspace`.
- [ ] 8.5 Run `make check`.
- [ ] 8.6 Remove `.compylr/` and `demo/demo-python-rust/.compylr/`, then run `make demo` and confirm
      no regression for programs that print nothing.
