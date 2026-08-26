## Why

The frontend computes the type of every expression and then throws it away.
`lower_expr` returns `(Expr, Option<Ty>)` — "shape and type produced together so they cannot
disagree" — and the type reaches nothing downstream. `Expr` carries no type, so the backend receives
only `expected: &Ty`, a **context** type pushed downward from a declaration, and has three narrow
ways to learn anything: that context, the declared types on `Bind`/`Assign`/`For`/`Param`/`ret`, and
a callee's signature via `unit.get`.

That absence is not a stylistic gap. It is the direct cause of four things already in the source:

* `rust.rs:1894` — reading a name **clones** whenever the context type is not trivially copyable,
  because the real type is a guess.
* `rust.rs:2046` — `Expr::Len` is a runtime trait dispatch under *every* mode, including modes where
  the answer is a plain `.len()`, because the backend cannot tell a collection from a string.
* `rust.rs:2205-2250` — an unchecked arithmetic operation has **three** emission paths rather than
  two, and the third exists only because under a comparison the context type is `Ty::Unit` and says
  nothing, so `a + b > c` has to compile for `i64` and `String` alike.
* Passing text as `&str` was built and **reverted**. `CLAUDE.md` records why in one sentence:
  *"Deciding this correctly needs the backend to know an expression's type, which it deliberately
  does not."*

Each of those is a workaround for the same missing fact, and each costs something real — a copy per
read, an O(1) operation emitted as a dispatch, a code path that exists to be wrong-proof rather than
right.

The comparison against `inspiration/py2many/` that prompted this change makes the alternative
concrete: it annotates every AST node with a type, and every one of its thirteen backends is
type-directed as a result — casts, container spellings, `&`/`&mut`, and width promotion all fall out
of a fact the tree carries. Its *representation* is bad (types as AST nodes compared by unparsing to
strings, joins as textual `Union[a, b]`) and is not what is proposed here. What it demonstrates is
that a backend that can ask an expression its type is a different kind of backend.

**Why now, and why exactly once.** This is an artifact-format change, so it costs every user one
rebuild. Paying that before the accepted subset grows is one rebuild; paying it after is the same
rebuild plus reworking every construct added in between. `add-python-backend` deliberately runs
first, so the question of whether the IR is genuinely target-neutral is answered by a second
consumer *before* the IR's shape is changed, and whatever that backend found is an input here.

## What Changes

- **BREAKING (IR shape). Every expression carries its type.** `Expr` becomes a node with a *form*
  and a *type*, so the two are constructed together and cannot disagree — the property `lower_expr`
  already maintains internally and then discards. The artifact format moves to version 5 and every
  existing `.compylr` cache rebuilds once, automatically, because build state records the compiler
  version.

- **A lowered expression always has a type.** Where inference cannot determine one — a call to a
  function this compilation cannot see — the annotation the subset already requires supplies it, and
  where neither does, that is the existing `UndeterminedBinding` diagnostic, unchanged. There is no
  "unknown" type: a variant meaning *undetermined* would be a variant every backend has to handle
  and none can emit.

- **Verification checks that the types agree.** A unit whose declared expression types are
  inconsistent with its operations is malformed in exactly the sense the verifier already exists to
  catch: it would produce target source that does not build, and the failure would arrive as a
  complaint about generated code rather than about the program.

- **The three workarounds go away.** `Expr::Len` emits directly where the type says it can; the
  `Ty::Unit`-under-comparison dispatch path is deleted; and reading a name copies only what actually
  needs copying. Each is a measurable change and each is measured.

- **`lower.rs` is split.** 3567 lines in one file currently hold name binding, signature collection,
  annotation lowering, statement lowering, expression lowering, and inference. The seams already
  exist — `collect_class_names`, `collect_signatures`, then body lowering — and this change is what
  makes them files. Threading a type through every expression touches all of it, so splitting during
  this change costs nothing and splitting after it costs a second pass over the same code.

- **Not in scope: borrowed parameters.** Knowing an expression's type is *necessary* for passing
  text as `&str` and is not *sufficient* — `CLAUDE.md` records four ordinary shapes that need an
  owned `String`, and `a_text_parameter_is_usable_in_every_position` exists because the whole suite
  passed while that was broken. It gets its own change, with that test as the gate.

- **No change to the accepted subset, to any diagnostic, or to any answer.** The programs compylr
  accepts and the results it produces are identical before and after.

## Capabilities

### Modified Capabilities
- `ir`: expression forms carry their type; the artifact format moves to version 5; the fingerprint
  covers the added information.
- `ir-lowering`: lowering SHALL produce a type for every expression it emits, and the existing rule
  that an undetermined initializer requires an annotation is what makes that always possible.
- `ir-optimization`: verification SHALL reject a unit whose expression types are inconsistent with
  its operations, and a pass SHALL leave the unit consistently typed.
- `rust-backend`: emission SHALL determine an expression's type by asking the expression, not by
  inferring it from context — and the operations that had to be conservative because it could not
  stop being conservative.

## Impact

**Modified**
- `crates/compylr-ir/src/ir.rs` — the expression node, the artifact version, the fingerprint.
- `crates/compylr-frontend-python/src/lower.rs` — split into files along its existing seams, and
  every expression construction carries its type.
- `crates/compylr-core/src/verify.rs` — the consistency check.
- `crates/compylr-core/src/folding.rs` — folding preserves the type it replaces.
- `crates/compylr-backend-rust/src/rust.rs` — type-directed emission; three workarounds removed.
- `crates/compylr-backend-python/src/` — the same, for the second backend.
- Every test that builds IR by hand: `conformance.rs`, `execution.rs`, `passes.rs`, and the IR unit
  tests. Constructor helpers that derive the type keep these from becoming unreadable — and keep a
  test from writing a tree whose type contradicts its form.
- `README.md` and `CLAUDE.md` — the format version, and the upgrade note.

**Unaffected**
- The Python package's surface, the decorator, settings, and every diagnostic message.
- `python/fixtures/` — no fixture changes, and the differential corpus from
  `add-differential-fixture-testing` is what proves no answer moved.

**Costs**
- One rebuild for every existing project, handled automatically by the recorded compiler version.
- Hand-built IR gets more verbose. Mitigated by helpers, not by making the type optional.
