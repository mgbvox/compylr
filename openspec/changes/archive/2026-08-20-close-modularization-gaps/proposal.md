## Why

`modularize-language-pipeline` made the IR's arithmetic semantics declarable and stopped there.
`Expr::Subscript` and `Expr::Len` still mean *Python's* — a negative index counts from the end,
`len` counts code points — with no way for another frontend to say otherwise, which is the exact
condition the change existed to remove. `runtime.rs` admits it in its own doc comment.

That half of the runtime is also the half with no native tests. Not a coincidence: `src/backend/
mod.rs` never declared the module, so the file with every semantics correction in it was compiled
only inside somebody else's generated crate until the workspace split. It sits at 57.95%.

Two more gaps the same verification pass turned up. The backend conformance corpus covers all 46 IR
node forms and still missed a real defect, because the defect was a form's behaviour in a
*constructor* and the corpus checked forms rather than positions. And `compylr compyle` cannot
import a package's `__init__.py` at all — it registers no parent for the private module names it
invents, so every relative import inside one fails.

## What Changes

- **BREAKING (IR shape).** `Expr::Subscript` carries an **index origin** and `Expr::Len` carries
  **text units**. Python declares counting from either end and counting code points; Go, C++, and
  TypeScript would declare otherwise. This changes the serialized IR, so the artifact format goes to
  version 3 and every cached build rebuilds once.
- The runtime's container helpers take the declared mode rather than assuming one, and the three
  places where languages genuinely disagree are the only ones parameterized. Missing-key behaviour,
  mapping iteration, and string membership stay as they are, each for a stated reason.
- **Every helper in the emitted runtime gains a native test.** Both index origins, all three text
  unit readings, mapping reads, element assignment, membership over all four containers, and
  iteration over a mapping.
- The conformance corpus is checked per `(form, emission context)` rather than per form, over the
  five contexts the backend renders differently: a function body, a constructor, a `&self` method, a
  `&mut self` method, and a loop body.
- `compylr compyle` imports packages the way the runtime does: a synthetic root package is
  registered, `__init__.py` is loaded as a genuine package, and missing ancestors are created on
  demand rather than relying on filename sort order.

## Capabilities

### New Capabilities

None. Every change here is to behaviour an existing capability already describes.

### Modified Capabilities

- `ir`: subscripting and length carry the semantics a frontend declared, alongside the arithmetic
  operators that already do.
- `python-frontend`: declares Python's container readings, as it already declares Python's
  arithmetic.
- `rust-backend`: reproduces the declared container semantics rather than Python's by name.
- `pipeline-architecture`: the conformance corpus covers emission contexts, not only node forms.
- `cli`: precompile discovery imports a package the way the runtime imports it.

## Impact

- **Caches.** Fingerprints move again; the build state's compiler-version check makes the rebuild
  automatic. This is the second forced rebuild in this line of work and is intended to be the last:
  the IR's remaining Python-specific behaviour after this change is documented as deliberate rather
  than pending.
- **Generated code.** `compat.rs` gains two mode enums and its helpers gain a parameter, so every
  project's emitted runtime changes. The emitted call sites change with it.
- **Coverage.** `runtime.rs` should clear ~90% from 57.95%.
- **Not in scope.** Splitting `Subscript` into sequence-index and mapping-lookup nodes, which would
  remove the inert-field compromise this change accepts; and any second frontend or backend.
