export const meta = {
  name: 'compylr-cpp-comprehensive-review',
  description: 'Audit compylr for false claims, research binding/transpiler prior art, and revise the C++ backend plan',
  phases: [
    { title: 'Guard', detail: 'live session-limit probe between segments', model: 'sonnet' },
    { title: 'Audit', detail: 'fan out over subsystems hunting claims that are not true' },
    { title: 'Verify', detail: 'adversarial refutation of every audit finding' },
    { title: 'Research', detail: 'web research on bindings, C++26, and transpiler prior art' },
    { title: 'PriorArt', detail: 'discover and document similar projects into ./inspiration' },
    { title: 'Synthesize', detail: 'three lenses then a merge', model: 'opus' },
    { title: 'Critique', detail: 'what did we miss', model: 'opus' },
  ],
}

const ROOT = '/Users/mgb/RustRoverProjects/compylr'

const RULES = [
  'REPO: ' + ROOT,
  '',
  'HARD CONSTRAINT — THIS IS PLANNING MODE. You are READ-ONLY on all project code.',
  'You MUST NOT create, edit, or delete anything under: crates/, frontends/, demo/, openspec/,',
  'scripts/, vendored/, Makefile, README.md, CLAUDE.md, Cargo.toml, pyproject.toml, or .github/.',
  'You MUST NOT run git commands that mutate state (commit, push, checkout, add, submodule add, reset).',
  'You MUST NOT run cargo build/test that modifies tracked files. Read-only cargo/CLI invocations',
  'for evidence (e.g. `cargo run -q -p compylr-cli -- --emit rust <file>`) ARE allowed and encouraged.',
  '',
  'YOU MAY WRITE ONLY TO: research/** (tracked, durable), context/** (gitignored scratch), and',
  'inspiration/*.md (summary docs). Findings, writeups and evidence transcripts go in research/ --',
  'context/ is wiped and untracked, and work has already been lost that way. Throwaway probe files',
  'and scratch build dirs belong in context/. Never write into inspiration/py2many/.',
  '',
  'If you need WebSearch or WebFetch, load them first with:',
  'ToolSearch({query: "select:WebSearch,WebFetch", max_results: 2})',
  '',
  'EVIDENCE DISCIPLINE — this matters more than volume:',
  '- Every claim needs a file:line citation or a command transcript you actually ran.',
  '- Never report something you did not verify. Mark anything unverified as UNVERIFIED explicitly.',
  '- Quote the code. A paraphrase of code is not evidence.',
  '- Prefer running the compiler over reading it when a behavior question can be answered by running it.',
].join('\n')

const CONTEXT = [
  'BACKGROUND — what compylr is and what has already been found.',
  '',
  'compylr transpiles a strict, fully annotated source subset to a compiled target and makes the result',
  'callable from the source language. Pipeline: source -> frontend -> IR -> verify -> passes -> backend',
  '-> target source -> host bridge -> loadable artifact. Frontends and backends are named components in',
  'compylr-registry and compose N+M. A HOST BRIDGE is keyed by the (source, target) PAIR and costs N x M.',
  'Today: frontends python + typescript; backends rust + go (cpp reserved); bridges (python,rust) via PyO3',
  'and (typescript,go) via cgo c-shared.',
  '',
  'IR operations carry the semantics the RESOLVED BEHAVIOR declared, across six axes: integer overflow,',
  'integer division, exact division, remainder, sequence indexing, text length. A backend matches on the',
  'MODES a node carries, never on the operation name. A GUARANTEE (overflow reported / div-by-zero',
  'reported / float order preserved) is what a frontend requires and a backend declares it preserves;',
  'core refuses an incompatible combination by name before emission.',
  '',
  'TWO CONFIRMED DEFECTS FOUND BEFORE THIS REVIEW — do not re-litigate, but DO look for more of the',
  'same CLASS, which is "a claim the repository makes that is not true":',
  '',
  'ISSUE #37 — the TypeScript frontend lowers `number` to Ty::Int (frontend-typescript/src/lower.rs:184)',
  'and `/` to integer division (lower.rs:1128 via behavior.integer_division(), which can only ever produce',
  'DivMode::Integer). Confirmed by running the CLI: `export function half(x: number): number { return x/2 }`',
  'emits `PyNum::div_trunc(&(x), &(2i64))`. half(5) is 2.5 in TypeScript; compiled it returns 2.',
  '',
  'ISSUE #38 — demo/demo-ts-go never runs compiled Go at all. _compylr.ts compyle() returns the function',
  'unchanged; nothing in demo/demo-ts-go/src references dlopen/ffi/.so/import(); .compylr/ has no lib/.',
  'Its benchmark.ts:155-160 takes ONE timing and computes the "interpreted" column as fast * a hardcoded',
  'per-item `speedup:` constant (benchmark.ts:35-149), so the README speedup column IS those constants.',
  'The reference row is fast * (1 +/- random 2%) presented as a measurement noise floor. And',
  'bridge.rs:74-76 in compylr-bridge-typescript-golang silently returns without emitting an export for',
  'ANY function whose params or return are non-scalar (is_scalar = Int|Float|Bool|Unit).',
  '',
  'THE PLAN UNDER REVIEW — PR #36, openspec/changes/add-cpp-backend/. It proposes a C++26 backend plus a',
  'shared C-ABI bridge crate (compylr-bridge-cpp-abi) with thin per-frontend loaders, on the theory that',
  'C++ is where the deferred canonical-C-ABI hub in crates/compylr-core/src/bridge.rs can be cashed in.',
  'THAT PREMISE HAS SINCE COLLAPSED: Node has no core FFI (node:ffi does not exist; process.dlopen only',
  'loads Node-API addons, not arbitrary C symbols), so the Node side needs Node-API regardless. The user',
  'has now decided: nanobind for Python->C++, node-addon-api for Node->C++, PyO3 stays for Python->Rust.',
  'Bridges therefore stay pairwise (N x M) and cpp-abi-bridge should be dropped from the plan.',
  '',
  'The user has also stated: EVERY (source, target) pair owes a working demo.',
  '',
  'FILED SINCE THE FIRST RUN — these are ALREADY REPORTED. Do not re-report them as new findings.',
  'Cite them where relevant, and look for what they MISSED:',
  'ISSUE #39 — the (typescript, go) bridge has never executed. 18 of 75 members exported (24%);',
  'zero classes (the bridge never calls Unit::classes()); index.js cannot be imported at all',
  '(uses require in a "type":"module" package, and koffi is not a dependency anywhere);',
  'emit_js_loader types every param and return int64 regardless of IR type, so Call_half(9)',
  'returns 0 where the correct ABI gives 4.5; a Go panic kills the host with SIGABRT and no error',
  'translation exists; a function named main/path/lib breaks both the emitted Go and the emitted JS;',
  'and compylr-host-typescript is an 8-line stub returning "0.1.0".',
  'ISSUE #40 — several checks cannot fail: `gofmt -l` exits 0 so make go-lint never fails;',
  'make ts-lint lints nothing; make go-test runs no tests (no *_test.go exists anywhere);',
  'make check omits CI\'s demo jobs and RUSTFLAGS=-D warnings; the pre-commit benchmark-markers',
  'hook `files:` pattern misses both files its own script reads; CLAUDE.md\'s whole Commands block',
  'still uses the pre-refactor python/ tree; and README.md says the Go backend is "not built yet"',
  'while the registry ships it.',
  'CORRECTION to #38: update_benchmarks.py DOES cover demo-ts-go — it derives the demo set by',
  'iterating demo/ (line 187). So the fabricated table is written BY the generated-docs mechanism,',
  'complete with a provenance line claiming it was measured. --check is marker-presence-only.',
].join('\n')

const FINDINGS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['dimension', 'findings', 'coverage_notes'],
  properties: {
    dimension: { type: 'string' },
    coverage_notes: { type: 'string', description: 'What you examined, and what you could NOT examine and why' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['title', 'severity', 'claim', 'evidence', 'why_it_matters', 'verified_how'],
        properties: {
          title: { type: 'string' },
          severity: { type: 'string', enum: ['critical', 'high', 'medium', 'low'] },
          claim: { type: 'string', description: 'The precise defect, in one sentence' },
          evidence: { type: 'string', description: 'file:line citations and/or a command transcript you ran' },
          why_it_matters: { type: 'string' },
          verified_how: { type: 'string', enum: ['ran-it', 'read-code-only', 'unverified'] },
        },
      },
    },
  },
}

const REFUTE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['verdicts'],
  properties: {
    verdicts: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['title', 'refuted', 'reasoning', 'corrected_claim'],
        properties: {
          title: { type: 'string', description: 'must match the finding title verbatim' },
          refuted: { type: 'boolean' },
          reasoning: { type: 'string' },
          corrected_claim: { type: 'string', description: 'if partially right, the accurate version; else empty' },
        },
      },
    },
  },
}

const RESEARCH_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['topic', 'summary', 'implications_for_compylr'],
  properties: {
    topic: { type: 'string' },
    summary: { type: 'string' },
    key_facts: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['fact'],
        properties: {
          fact: { type: 'string' },
          source_url: { type: 'string' },
          confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
        },
      },
    },
    implications_for_compylr: { type: 'string' },
    open_questions: { type: 'string' },
    context_file: { type: 'string', description: 'path of the markdown file you wrote under research/' },
  },
}

const PROJECTS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['projects'],
  properties: {
    projects: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['name', 'repo_url', 'what_it_does', 'relevance', 'relevance_score', 'lessons'],
        properties: {
          name: { type: 'string' },
          repo_url: { type: 'string' },
          what_it_does: { type: 'string' },
          relevance: { type: 'string', description: 'how it relates to compylr specifically' },
          relevance_score: { type: 'integer', minimum: 1, maximum: 10 },
          lessons: { type: 'string', description: 'what compylr should copy or avoid' },
        },
      },
    },
  },
}

const CAPTURE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['name', 'doc_path', 'verdict', 'should_vendor', 'summary'],
  properties: {
    name: { type: 'string' },
    doc_path: { type: 'string' },
    verdict: { type: 'string', enum: ['high-value', 'moderate', 'low-value', 'not-relevant'] },
    should_vendor: { type: 'boolean', description: 'true if worth adding as a git submodule under inspiration/' },
    summary: { type: 'string' },
  },
}

// ---------------------------------------------------------------- audit

const AUDIT_DIMENSIONS = [
  {
    key: 'ts-frontend',
    focus: [
      'The TypeScript frontend, crates/compylr-frontend-typescript/. Issue #37 confirmed `/` is wrong.',
      'Find EVERY OTHER semantic divergence between what TypeScript means and what this frontend lowers.',
      'Specifically probe, by RUNNING the CLI (`cargo run -q -p compylr-cli -- --frontend typescript --emit rust <f>`):',
      'non-integral number literals; % on non-integral operands; ** ; comparison and equality semantics;',
      'string indexing and .length (UTF-16 code units in JS vs bytes vs code points); array methods;',
      'Map/Set iteration; truthiness in conditions; ?? and ||; bigint; Number overflow beyond 2^53;',
      'and whether `number` should map to Ty::Float or whether a distinct annotation is needed',
      '(lower.rs:215 already accepts "int" and "float" as named types, which suggests intent).',
      'Also check whether the TypeScript frontend declares a behavior stance at all, and whether that',
      'declaration matches JS reality on all six axes.',
    ].join(' '),
  },
  {
    key: 'ts-go-bridge',
    focus: [
      'crates/compylr-bridge-typescript-golang/. Issue #38 confirmed emit_cgo_function silently skips',
      'non-scalar signatures. Determine EXACTLY what fraction of the IR this bridge can actually export,',
      'what it emits for classes (if anything), whether the emitted cgo/JS actually compiles and loads,',
      'whether index.js/index.d.ts reference symbols that are never exported, and whether HostArtifact.manifest',
      'and loaded_as are correct. Then judge: is the (typescript, go) pair functional in any real sense?',
      'Read crates/compylr-bridge-typescript-golang/tests/bridge.rs and assess whether those tests could',
      'ever have caught the silent skip.',
    ].join(' '),
  },
  {
    key: 'go-backend',
    focus: [
      'crates/compylr-backend-golang/. Does emitted Go actually implement the modes each IR node carries,',
      'or does it read operation names? Check every one of the six axes against src/compat.rs and src/emit.rs.',
      'Does it handle every Stmt and Expr form, or does it silently skip/emit-nothing for some (the same',
      'failure class as the bridge)? Does GO_BEHAVIOR match real Go semantics? Does PRESERVES match what',
      'compat.rs can actually deliver? Does the emitted Go compile? Try emitting a fixture and running gofmt/go vet.',
      'Compare its completeness honestly against crates/compylr-backend-rust/.',
    ].join(' '),
  },
  {
    key: 'demo-integrity',
    focus: [
      'Both demos. Issue #38 covers demo-ts-go benchmark fabrication. Now check EVERYTHING ELSE:',
      'do demo/demo-ts-go/tests/*.test.ts actually assert real behavior or are they vacuous?',
      'Does its ir_coverage.ts do what demo-python-rust/ir_coverage.py does, or is it a stub?',
      'Does its README make other claims that are not true?',
      'Then audit demo/demo-python-rust the SAME way with fresh eyes: is its benchmark real (two processes,',
      'real timings)? Is its coverage claim actually asserted? Does COMPYLR_DISABLE genuinely produce an',
      'interpreted run? Is anything there also simulated? Do not assume the Python demo is honest because',
      'it is older — verify it.',
    ].join(' '),
  },
  {
    key: 'enforcement-tests',
    focus: [
      'The tests in crates/compylr-host-python/tests/ that the repo relies on as ENFORCEMENT:',
      'crate_boundaries.rs, conformance.rs, fixtures.rs, emit_quality.rs, readme.rs, demo_coverage.rs,',
      'differential.rs, corpus.rs. For each: does it actually enforce what its name and doc-comment claim?',
      'Look specifically for: early returns / continues that skip cases; hardcoded lists that should be',
      'derived; #[ignore] attributes; assertions that cannot fail; loops over empty collections; filters',
      'that exclude the interesting case. conformance.rs claims to run every corpus entry through every',
      'implemented backend — verify that the Go backend is genuinely exercised and that failures would fail.',
      'differential.rs — does it cover the TypeScript/Go pair at all, or only Python/Rust?',
    ].join(' '),
  },
  {
    key: 'python-rust-path',
    focus: [
      'The mature path: crates/compylr-frontend-python/, crates/compylr-backend-rust/,',
      'crates/compylr-bridge-python-rust/, frontends/python/compylr/. Hunt for the SAME class of defect',
      'found elsewhere — silent skips, unreachable branches, claims in doc comments that the code does not',
      'honor, capabilities asserted in CLAUDE.md that do not hold. Check that every Ty and every Expr/Stmt',
      'form actually round-trips through the PyO3 bridge (bindings.rs emit_boundary_type / emit_function_param),',
      'and that nothing is silently dropped the way the cgo bridge drops non-scalars.',
      'Verify by running the CLI on fixtures where practical.',
    ].join(' '),
  },
  {
    key: 'spec-vs-reality',
    focus: [
      'openspec/specs/ against the code. For each capability spec — especially golang-backend,',
      'typescript-frontend, typescript-api, typescript-bindings, typescript-go-bridge, demo, fixture-corpus,',
      'pipeline-architecture, semantic-behavior — list requirements whose scenarios are NOT satisfied by the',
      'code as it stands. The typescript-* and golang-backend specs were written alongside the ts-go change',
      'that we now know shipped partly non-functional, so treat their claims as suspect and check each one.',
      'Report the specific requirement heading and the specific reason it is not met.',
    ].join(' '),
  },
  {
    key: 'generated-docs',
    focus: [
      'scripts/update_benchmarks.py, scripts/update_subset.py, scripts/_regions.py, and the marker regions',
      'they claim to own. Determine exactly which README regions in the repo are ACTUALLY generated and',
      '--check-guarded, and which regions look generated (have markers) but are not covered by any script.',
      'Confirm the demo-ts-go finding and find any others. Also check .github/workflows/ and',
      '.pre-commit-config.yaml and the Makefile: do CI, hooks, and `make check` really run the same commands',
      'CLAUDE.md says they do? List every divergence. Check whether the TypeScript and Go suites in `make check`',
      'actually assert anything (e.g. `make go-lint` runs gofmt -l which does not fail on output).',
    ].join(' '),
  },
]

const PRIOR = {
  'ts-go-bridge': [
    '',
    'HEAD START — a previous agent already audited this dimension and its evidence file survived at',
    'research/audit-ts-go-bridge.md. READ IT FIRST. Its findings became issue #39. Your job is NOT to',
    'redo it: verify a sample of its transcripts still reproduce, then push PAST it. Specifically it',
    'did NOT examine: whether the Go backend emits correct code for the 57 members that never reach',
    'the boundary; whether HostArtifact.manifest and loaded_as are correct; what happens to Str',
    'parameters specifically; and whether any of this is recoverable incrementally or needs a rewrite.',
  ].join(' '),
  'generated-docs': [
    '',
    'HEAD START — a previous agent already audited this dimension and its evidence file survived at',
    'research/audit-generated-docs.md. READ IT FIRST. Its findings became issue #40 and the correction',
    'on #38. Your job is NOT to redo it: verify a sample still reproduces, then push PAST it.',
    'Specifically it did NOT examine: the Python demo\'s benchmark for the same class of defect with',
    'fresh eyes; whether scripts/update_subset.py\'s generated subset matrix is actually true against',
    'the corpus; and whether .github/workflows/ jobs can fail for the reasons they claim to check.',
  ].join(' '),
}

function auditPrompt(d) {
  return [
    RULES,
    '',
    CONTEXT,
    '',
    '=== YOUR ASSIGNMENT: audit dimension "' + d.key + '" ===',
    '',
    d.focus,
    PRIOR[d.key] || '',
    '',
    'You are hunting for ONE class of defect above all: A CLAIM THE REPOSITORY MAKES THAT IS NOT TRUE.',
    'That includes code that silently does nothing, tests that cannot fail, documentation asserting',
    'behavior that does not exist, and measurements that are not measurements.',
    '',
    'Be exhaustive within your dimension. Run the compiler to check behavior wherever you can rather than',
    'reasoning from the source. Write any long evidence transcripts to research/audit-' + d.key + '.md.',
    'Rate severity by how misleading the claim is, not by how hard it is to fix: something that makes a',
    'broken thing look working is critical; a wrong number is high; a cosmetic gap is low.',
    'Report ONLY defects you actually confirmed. An empty findings list is a fine and honest answer.',
  ].join('\n')
}

function refutePrompt(d, findings, lens) {
  const lensText = {
    correctness: 'Attack the technical accuracy. Is the code actually doing what the finding says? Re-read the cited lines yourself. Run the command yourself. Is there a code path the finder missed that makes the behavior correct after all?',
    intent: 'Attack the interpretation. Is this actually a defect, or is it a documented, deliberate design decision? Check CLAUDE.md, the module doc comments, openspec/specs/, and the archived changes under openspec/changes/archive/ for a stated rationale. compylr deliberately has several behaviors that look like bugs until you know why.',
    materiality: 'Attack the significance. Even if technically accurate, does it matter? Is the code path reachable by a real user? Is the claim it contradicts actually made anywhere load-bearing? Would fixing it change any observable behavior?',
  }[lens]
  return [
    RULES,
    '',
    CONTEXT,
    '',
    '=== YOUR ASSIGNMENT: adversarially REFUTE these findings ===',
    '',
    'Another agent audited "' + d.key + '" and produced the findings below. Your job is to KILL them.',
    'You are not a reviewer looking for balance — you are a skeptic trying to prove each one wrong.',
    '',
    'YOUR LENS: ' + lens + '. ' + lensText,
    '',
    'Verify independently. Do not trust the finder\'s citations — open the files and check the line numbers',
    'yourself, and re-run any command they claim to have run. A finding whose cited evidence does not say',
    'what the finder claims it says is REFUTED.',
    '',
    'Default to refuted=true when you are genuinely uncertain. Set refuted=false only when you have',
    'independently confirmed the finding is real, accurate, and matters.',
    'If a finding is partly right, set refuted=false and put the accurate narrower version in corrected_claim.',
    '',
    'Return one verdict per finding, with `title` matching the finding title VERBATIM.',
    '',
    'FINDINGS TO REFUTE:',
    JSON.stringify(findings, null, 2),
  ].join('\n')
}

// ---------------------------------------------------------------- research

const RESEARCH_TOPICS = [
  {
    key: 'nanobind',
    q: 'nanobind for Python<->C++ bindings: current maturity and version; minimum C++ standard required; Python version support and stable-ABI (abi3 / Py_LIMITED_API) story and which Python versions it covers; compile-time and binary-size versus pybind11 with real numbers; per-call overhead; STL container conversion support and its copying semantics; class/instance binding (nb::class_) and ownership models; how it is consumed by a build (CMake FetchContent? pip package providing headers? nanobind_add_module); known limitations and gotchas. Cite the official docs and the author\'s published benchmarks.',
  },
  {
    key: 'python-cpp-alternatives',
    q: 'Comparison of Python<->C++ binding approaches as of 2026: pybind11, nanobind, cppyy, SWIG, Cython, Boost.Python, and raw CPython C API. For each: maintenance status, build-time cost, runtime call overhead, ABI stability, and whether it suits CODE-GENERATED bindings specifically (as opposed to hand-written ones). Which are best for a compiler that emits binding source automatically? Find real benchmark data where it exists.',
  },
  {
    key: 'python-call-overhead',
    q: 'Hard numbers on per-call overhead crossing from Python into native code: ctypes vs cffi (ABI mode and API mode) vs PyO3 vs pybind11 vs nanobind vs a hand-written CPython C extension. Nanosecond-scale benchmarks, ideally with methodology. How much does argument marshalling of a list/dict of N elements cost in each? This is the decisive empirical question for whether a C-ABI-everywhere design is viable for Python.',
  },
  {
    key: 'node-api',
    q: 'Node-API (N-API) and node-addon-api for calling C++ from Node.js: the ABI stability guarantee across Node major versions and how NAPI_VERSION works; node-addon-api vs raw Node-API C; building with cmake-js versus node-gyp and which suits a project that already emits CMakeLists.txt; Napi::ObjectWrap for wrapping native instances; error and exception propagation; typed arrays and zero-copy; performance/call overhead; prebuild and distribution (prebuildify, node-pre-gyp). Also: does Node have ANY core FFI, and what is the status of ffi-napi, koffi, and Bun/Deno FFI as alternatives?',
  },
  {
    key: 'cpp26-support',
    q: 'C++26 compiler support status as of mid-2026: which features GCC 15/16, Clang 20/21, and MSVC actually implement; whether -std=c++26 is usable in production; the status of contracts, static reflection, and std::execution; and separately which compiler versions provide std::expected (C++23), deducing this, and <stdckdint.h> / checked integer arithmetic. Is targeting -std=c++26 while only using C++23-era features a sound strategy for generated code? Cite cppreference compiler support tables and the GCC/Clang release notes.',
  },
  {
    key: 'universal-bindings',
    q: 'Projects that generate bindings for MANY host languages from one definition, i.e. that turn an N x M binding problem into N + M: Mozilla uniffi, Diplomat, cbindgen, flapigen, SWIG, the WebAssembly Component Model and WIT, GraalVM polyglot, and .NET P/Invoke source generators. For each: what the intermediate description is, which host languages it reaches, what it costs in performance and expressiveness, and how it handles object identity, mutation, and error propagation across the boundary. Is a canonical-C-ABI hub actually how successful projects solve this, or do they do something else?',
  },
  {
    key: 'python-native-compilers',
    q: 'Projects that compile Python to native code: Nuitka, Cython, mypyc, Codon, Numba, Pythran, Shed Skin, and the Python 3.13+ JIT. For each: what subset of Python it accepts, how it handles integer semantics (arbitrary precision, // vs /, overflow) versus machine integers, how it handles the boundary back into CPython, and what speedups are actually reported. Which of them made the same design choices compylr is making, and what did they learn?',
  },
  {
    key: 'multi-target-transpilers',
    q: 'Source-to-source compilers with MULTIPLE target languages driven by one IR: py2many, Haxe, Nim, Transcrypt, J2ObjC, and MLIR/LLVM as a comparison point. How do they structure frontend/IR/backend separation? How do they handle semantics that differ between targets (integer division rounding, overflow, indexing, string length units)? Does any of them have an explicit per-operation semantics negotiation like compylr behavior axes, or is that unusual? What breaks for them at scale?',
  },
  {
    key: 'ts-native',
    q: 'Compiling TypeScript or JavaScript to native code: AssemblyScript, Porffor, Static Hermes, QuickJS/quickjs-ng, Bun\'s bundler+compile, and any TypeScript-to-C++ projects. How does each handle the fact that JS `number` is IEEE-754 double and there is no integer type? Do any of them recover integer types by inference or annotation, and how? This bears directly on whether compylr should map TypeScript `number` to Int, Float, or something else.',
  },
  {
    key: 'semantics-mismatch',
    q: 'How do transpilers and cross-language compilers handle semantic mismatch between source and target — specifically integer division rounding (floor vs truncate), remainder sign, integer overflow (wrap vs trap vs promote), sequence indexing (negative indices, bounds checking), and string length units (bytes vs UTF-16 code units vs code points)? Find concrete examples of projects that got this wrong and what the bug reports looked like. Is there published work or prior art on making these choices explicit and configurable per operation?',
  },
]

function researchPrompt(t) {
  return [
    RULES,
    '',
    CONTEXT,
    '',
    '=== YOUR ASSIGNMENT: research "' + t.key + '" ===',
    '',
    t.q,
    '',
    'Load WebSearch and WebFetch first. Search broadly, then FETCH the primary sources — official docs,',
    'release notes, repository READMEs, benchmark pages — rather than relying on search snippets or on',
    'your own recollection. Your training data may be stale; the web is the authority here.',
    '',
    'Prefer specific numbers, version numbers, and dates over qualitative claims. Where you cannot find a',
    'number, say so rather than estimating one. Mark confidence honestly: `high` only when you fetched a',
    'primary source that states it directly.',
    '',
    'Write a full markdown writeup to research/' + t.key + '.md — the structured return value is a',
    'summary, the file is the record. Include source URLs inline in that file.',
    'Return context_file as the path you wrote.',
  ].join('\n')
}

function deepenPrompt(t, prior) {
  return [
    RULES,
    '',
    '=== YOUR ASSIGNMENT: adversarially deepen the research on "' + t.key + '" ===',
    '',
    'Another agent researched this topic and produced the summary below. Your job is to find what it got',
    'WRONG or MISSED, and to fill the gaps — not to restate it.',
    '',
    'Specifically:',
    '1. Check its key_facts. Fetch the cited sources. Does each source actually say what the fact claims?',
    '   Any fact whose source does not support it must be corrected or dropped.',
    '2. Find what it did not look for. Which sub-questions in the original topic went unanswered?',
    '3. Hunt for contradicting evidence. If it reports a benchmark, find a competing benchmark. If it',
    '   reports a capability, find the issue tracker complaints about that capability.',
    '4. Check recency. Is anything it reports superseded by a newer release?',
    '',
    'Load WebSearch and WebFetch first. APPEND your corrections and additions to',
    'research/' + t.key + '.md under a heading "## Adversarial review and gaps", preserving what is',
    'already there.',
    '',
    'Return the CONSOLIDATED picture — the original findings as corrected by you, plus what you added.',
    'implications_for_compylr should be your own independent judgement, not the prior agent\'s.',
    '',
    'PRIOR RESEARCH TO CHECK AND EXTEND:',
    JSON.stringify(prior, null, 2),
  ].join('\n')
}

// ---------------------------------------------------------------- prior art

const PRIOR_ART_ANGLES = [
  {
    key: 'python-transpilers',
    q: 'Find repositories that transpile or compile Python to other languages or to native code. Include py2many (already vendored), Nuitka, Cython, mypyc, Codon, Shed Skin, Pythran, Transcrypt, RPython/PyPy, Numba, Taichi, and anything less well known you can surface. Also look for decorator-driven "compile this function" projects specifically, since that is compylr\'s exact UX.',
  },
  {
    key: 'multi-backend-ir',
    q: 'Find repositories built around a language-neutral IR with pluggable frontends AND pluggable backends, especially source-to-source ones rather than machine-code compilers. Haxe, Nim, MLIR-based transpilers, Nanopass-style frameworks, universal-transpiler projects, and any research compilers with this architecture. Also find projects that explicitly model per-operation semantic differences between languages.',
  },
  {
    key: 'binding-generators',
    q: 'Find repositories that automatically generate host-language bindings for compiled code: uniffi, diplomat, cbindgen, flapigen, PyO3, nanobind, pybind11, node-addon-api, napi-rs, SWIG, wasm-bindgen, and the WIT/Component Model tooling. Focus on ones that target MULTIPLE host languages from one description, since that is the N x M problem compylr is deciding about right now.',
  },
]

function discoverPrompt(a) {
  return [
    RULES,
    '',
    CONTEXT,
    '',
    '=== YOUR ASSIGNMENT: discover prior art, angle "' + a.key + '" ===',
    '',
    a.q,
    '',
    'Load WebSearch and WebFetch first. Search GitHub and the web. For each project, FETCH the repository',
    'README rather than relying on your recollection of it, and record the real repo URL.',
    '',
    'Score relevance 1-10 by how much compylr could learn from it SPECIFICALLY. A wildly popular project',
    'that shares no design problem with compylr scores low; an obscure one that solved exactly compylr\'s',
    'frontend/IR/backend/bridge problem scores high. Be discriminating — a list of everything is useless.',
    '',
    'Note that inspiration/py2many is already vendored as a git submodule; include it only if you have',
    'something new to say about it.',
    '',
    'Return at least 8 projects for your angle, ranked.',
  ].join('\n')
}

function capturePrompt(p) {
  return [
    RULES,
    '',
    CONTEXT,
    '',
    '=== YOUR ASSIGNMENT: document prior-art project "' + p.name + '" ===',
    '',
    'Repo: ' + p.repo_url,
    'Why it surfaced: ' + p.relevance,
    '',
    'Investigate it properly. Load WebSearch and WebFetch. Fetch the README, the docs, and where it helps,',
    'specific source files via the raw GitHub URL. If it is small and clearly relevant you may `git clone`',
    'it into context/clones/ for reading — but NEVER into inspiration/ and never as a submodule.',
    '',
    'Answer, concretely:',
    '- What problem does it solve, and how does its architecture compare to compylr\'s',
    '  frontend / IR / backend / host-bridge split?',
    '- How does it handle cross-language semantic mismatch (integer division, overflow, indexing,',
    '  string length)? Does it have anything like compylr\'s behavior axes, or does it just pick one?',
    '- How does it make compiled code callable from the source language, and what did that cost it?',
    '- What is its accepted subset, and how does it communicate rejection to the user?',
    '- What can compylr STEAL from it, and what mistake of theirs should compylr avoid?',
    '',
    'Write inspiration/' + p.name.toLowerCase().replace(/[^a-z0-9]+/g, '-') + '.md with that analysis.',
    'Start the file with a one-line description, the repo URL, and the license.',
    'Set should_vendor true only if the repo is genuinely worth adding as a git submodule for ongoing',
    'reference the way py2many is — be strict, most are not.',
  ].join('\n')
}

// ---------------------------------------------------------------- usage guard
//
// One number and one timestamp:
//
//     GET https://claude.ai/api/organizations/{org_uuid}/usage
//     -> { "five_hour": { "utilization": 0-100, "resets_at": "<iso8601>" }, ... }
//
// `five_hour.utilization` IS the session-limit bar the Claude settings page draws. At or above
// PAUSE_AT we stop launching segments and hand back `resets_at` so the caller can arm a resume.
//
// Everything this replaced -- empirical token ceilings, Opus weighting, overage tiers, summing
// message.usage out of ~/.claude/projects -- was scaffolding for not being able to see this
// number. It is all gone on purpose. Do not reintroduce a token estimate: it was wrong twice
// (once by inventing a ceiling, once by reading a stale overage flag out of a log record and
// reporting $0 while real spend was accruing).
//
// TRANSPORT. The endpoint is cookie-authenticated and the session cookie is httpOnly, so the
// probe cannot curl it. It goes through the user's own logged-in Chrome via the claude-in-chrome
// MCP and runs fetch() in the page's context, where the cookies already apply -- no credential is
// ever read, copied, or handled. Verified working from inside a workflow subagent
// (run wf_6cacbc93-4b2: ok=true, utilization=100, 6 tool calls, 23s).
//
// LIMITATION. This needs a live, logged-in Chrome, so it does not work headless or under cron.
// A headless guard would need the sessionKey cookie supplied out of band.

const CFG = (typeof args === 'object' && args) ? args : {}
//: Pause when the session bar is this full. "Within 5%" of the limit.
const PAUSE_AT = CFG.pauseAtUtilization || 95

const PROBE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['ok', 'utilization', 'resets_at_iso', 'method'],
  properties: {
    ok: { type: 'boolean', description: 'true only on a real 200 from the live endpoint' },
    utilization: { type: 'number', description: 'five_hour.utilization 0-100; -1 when unknown' },
    resets_at_iso: { type: 'string', description: 'five_hour.resets_at verbatim, or ""' },
    method: { type: 'string', description: 'how it was obtained, naming any step that failed' },
  },
}

function probePrompt(where) {
  return [
    'Read the LIVE Claude session-limit utilization. It must come from the live claude.ai API —',
    'never from a local log file, and never from your own recollection. Do not read, print, or',
    'copy any credential; the browser holds the session and that is where the request belongs.',
    '',
    'METHOD, exactly:',
    '1. Load the browser tools in ONE call:',
    '   ToolSearch({query: "select:mcp__claude-in-chrome__tabs_context_mcp,mcp__claude-in-chrome__tabs_create_mcp,mcp__claude-in-chrome__navigate,mcp__claude-in-chrome__javascript_tool,mcp__claude-in-chrome__tabs_close_mcp", max_results: 5})',
    '2. tabs_context_mcp({createIfEmpty: true}) for a tabId.',
    '3. navigate that tab to https://claude.ai/new — the app must load for cookies to apply.',
    '4. javascript_tool on that tab, this exact code:',
    '',
    '     const o = await (await fetch("/api/organizations", {credentials:"include"})).json();',
    '     const u = await (await fetch(`/api/organizations/${o[0].uuid}/usage`, {credentials:"include"})).json();',
    '     JSON.stringify({utilization: u.five_hour.utilization, resets_at: u.five_hour.resets_at})',
    '',
    '5. tabs_close_mcp the tab you created. Always, even on failure.',
    '',
    'Return five_hour.utilization and five_hour.resets_at verbatim.',
    '',
    'If the page redirects to /login the browser is not authenticated: return ok=false,',
    'utilization=-1, resets_at_iso="", and say so in `method`. Same for any other failure.',
    'NEVER invent a number, and never report a high utilization you did not read — a failed',
    'measurement must not halt work.',
    '',
    'Checkpoint: ' + where,
  ].join('\n')
}

let paused = false
let pauseInfo = null

//: A segment where most agents returned null is systemic failure, not bad luck. agent() returns
//: null on a terminal API error, which is what a rate limit looks like from inside the script.
//: This is the one signal that does NOT depend on the probe being able to run.
function checkSegmentHealth(where, results) {
  const total = results.length
  if (!total) return false
  const dead = results.filter(r => !r).length
  const pct = Math.round((dead / total) * 100)
  if (dead) log('Segment "' + where + '": ' + dead + '/' + total + ' agents returned nothing (' + pct + '%).')
  if (dead / total < 0.3) return false
  paused = true
  pauseInfo = { at: where, utilization: -1, resets_at_iso: '', threshold: PAUSE_AT,
                reason: 'segment-failure-rate', detail: dead + '/' + total + ' agents died (' + pct + '%)' }
  log('PAUSING: ' + pct + '% of segment "' + where + '" died. That is systemic — almost certainly the '
    + 'session limit — so later segments were not launched.')
  return true
}

async function guard(where) {
  if (paused) return true
  const u = await agent(probePrompt(where), {
    label: 'usage-probe:' + where, phase: 'Guard', schema: PROBE_SCHEMA, model: 'sonnet', effort: 'low',
  })
  let reading = u
  if (!reading || !reading.ok || typeof reading.utilization !== 'number' || reading.utilization < 0) {
    log('Usage probe unreadable at "' + where + '" (' + (u ? u.method : 'agent returned nothing') + ') — retrying once.')
    reading = await agent(probePrompt(where + ':retry'), {
      label: 'usage-probe:' + where + ':retry', phase: 'Guard', schema: PROBE_SCHEMA, model: 'sonnet', effort: 'low',
    })
  }
  if (!reading || !reading.ok || typeof reading.utilization !== 'number' || reading.utilization < 0) {
    // FAIL CLOSED. The probe is itself an agent, so the most likely reason it cannot run is that
    // we are already rate limited -- exactly the condition it exists to catch.
    paused = true
    pauseInfo = { at: where, utilization: -1, resets_at_iso: '', threshold: PAUSE_AT,
                  reason: 'probe-unreadable', detail: reading ? reading.method : 'agent returned nothing' }
    log('PAUSING: the usage probe could not be read twice at "' + where + '". Treating an unreadable '
      + 'probe as a stop, not a green light — it is an agent, so the likeliest cause is that we are '
      + 'already over the limit. Later segments were not launched.')
    return true
  }
  const u2 = reading
  log('Session limit at "' + where + '": ' + u2.utilization + '% used'
    + (u2.resets_at_iso ? ', resets ' + u2.resets_at_iso : '') + '.')
  if (u2.utilization < PAUSE_AT) return false
  paused = true
  pauseInfo = { at: where, utilization: u2.utilization, resets_at_iso: u2.resets_at_iso, threshold: PAUSE_AT, reason: 'over-threshold' }
  log('PAUSING: session limit ' + u2.utilization + '% >= ' + PAUSE_AT + '%. Segments after "' + where
    + '" were not launched. Resume after ' + (u2.resets_at_iso || 'the window resets')
    + ' with the same scriptPath + resumeFromRunId; completed agents replay from cache.')
  return true
}

// ---------------------------------------------------------------- run

log('Segmented run. Pausing when the live session-limit bar reaches ' + PAUSE_AT + '%.')

const auditFindings = []
const researchRound1 = []
const discovered = []

// --- Segment 1: audit finders + research round 1 + prior-art discovery, all concurrent.
if (!await guard('start')) {
  const seg1 = await parallel([
    ...AUDIT_DIMENSIONS.map(d => () =>
      agent(auditPrompt(d), { label: 'audit:' + d.key, phase: 'Audit', schema: FINDINGS_SCHEMA, model: 'sonnet', effort: 'high' })
        .then(r => ({ kind: 'audit', d, r }))),
    ...RESEARCH_TOPICS.map(t => () =>
      agent(researchPrompt(t), { label: 'research:' + t.key, phase: 'Research', schema: RESEARCH_SCHEMA, model: 'sonnet', effort: 'high' })
        .then(r => ({ kind: 'research', t, r }))),
    ...PRIOR_ART_ANGLES.map(a => () =>
      agent(discoverPrompt(a), { label: 'discover:' + a.key, phase: 'PriorArt', schema: PROJECTS_SCHEMA, model: 'sonnet', effort: 'high' })
        .then(r => ({ kind: 'discover', a, r }))),
  ])
  checkSegmentHealth('segment-1', seg1)
  for (const item of seg1.filter(Boolean)) {
    if (!item.r) continue
    if (item.kind === 'audit') auditFindings.push({ d: item.d, found: item.r })
    if (item.kind === 'research') researchRound1.push({ t: item.t, prior: item.r })
    if (item.kind === 'discover') discovered.push(item.r)
  }
}

// --- Segment 2: adversarial refutation of everything segment 1 found.
const audit = []
if (!await guard('after-discovery')) {
  const refuted = await parallel(auditFindings.flatMap(({ d, found }) =>
    (found.findings && found.findings.length)
      ? ['correctness', 'intent', 'materiality'].map(lens => () =>
          agent(refutePrompt(d, found.findings, lens), { label: 'refute:' + d.key + ':' + lens, phase: 'Verify', schema: REFUTE_SCHEMA, model: 'sonnet', effort: 'high' })
            .then(v => ({ key: d.key, v })))
      : []
  ))
  checkSegmentHealth('segment-2-refutation', refuted)
  for (const { d, found } of auditFindings) {
    const votes = refuted.filter(Boolean).filter(x => x.key === d.key).map(x => x.v).filter(Boolean)
    audit.push({ dimension: d.key, findings: found.findings || [], coverage_notes: found.coverage_notes, votes })
  }
} else {
  for (const { d, found } of auditFindings) {
    audit.push({ dimension: d.key, findings: found.findings || [], coverage_notes: found.coverage_notes, votes: [] })
  }
}

// --- Segment 3: research deepening + prior-art capture.
let research = researchRound1.map(x => x.prior)
let priorArt = { ranked: [], captured: [] }
{
  const seen = new Map()
  for (const r of discovered) {
    for (const p of (r.projects || [])) {
      const k = (p.name || '').toLowerCase().trim()
      if (!k) continue
      const prev = seen.get(k)
      if (!prev || p.relevance_score > prev.relevance_score) seen.set(k, p)
    }
  }
  const ranked = Array.from(seen.values()).sort((a, b) => b.relevance_score - a.relevance_score)
  priorArt.ranked = ranked
  const top = ranked.slice(0, 10)
  if (!await guard('after-refutation')) {
    log('Prior art: ' + ranked.length + ' distinct projects; deep-diving the top ' + top.length + '. Dropped ' + Math.max(0, ranked.length - top.length) + '.')
    const seg3 = await parallel([
      ...researchRound1.map(({ t, prior }) => () =>
        agent(deepenPrompt(t, prior), { label: 'deepen:' + t.key, phase: 'Research', schema: RESEARCH_SCHEMA, model: 'sonnet', effort: 'high' })
          .then(r => ({ kind: 'deepen', r }))),
      ...top.map(p => () =>
        agent(capturePrompt(p), { label: 'capture:' + p.name, phase: 'PriorArt', schema: CAPTURE_SCHEMA, model: 'sonnet', effort: 'high' })
          .then(r => ({ kind: 'capture', r }))),
    ])
    checkSegmentHealth('segment-3', seg3)
    const deepened = seg3.filter(Boolean).filter(x => x.kind === 'deepen' && x.r).map(x => x.r)
    if (deepened.length) research = deepened
    priorArt.captured = seg3.filter(Boolean).filter(x => x.kind === 'capture' && x.r).map(x => x.r)
  }
}

// Fold the adversarial votes: a finding survives only if it is NOT refuted by a majority of lenses.
const auditSummary = audit.map(a => {
  const byTitle = new Map()
  for (const v of (a.votes || [])) {
    for (const verdict of (v.verdicts || [])) {
      const list = byTitle.get(verdict.title) || []
      list.push(verdict)
      byTitle.set(verdict.title, list)
    }
  }
  const survived = []
  const killed = []
  const unverified = []
  for (const f of (a.findings || [])) {
    const verdicts = byTitle.get(f.title) || []
    const refutes = verdicts.filter(v => v.refuted).length
    const entry = { finding: f, verdicts, refuted_count: refutes, total_votes: verdicts.length }
    if (verdicts.length === 0) unverified.push(entry)          // nobody voted -- NOT the same as surviving
    else if (refutes > verdicts.length / 2) killed.push(entry)
    else survived.push(entry)
  }
  return { dimension: a.dimension, coverage_notes: a.coverage_notes, survived, killed, unverified }
})

const totalSurvived = auditSummary.reduce((n, a) => n + a.survived.length, 0)
const totalKilled = auditSummary.reduce((n, a) => n + a.killed.length, 0)
const totalUnverified = auditSummary.reduce((n, a) => n + a.unverified.length, 0)
log('Audit: ' + totalSurvived + ' findings survived adversarial review, ' + totalKilled + ' refuted, '
  + totalUnverified + ' NEVER VERIFIED (no refuter ran — treat these as unconfirmed claims).')

const dossier = JSON.stringify({ audit: auditSummary, research, prior_art: priorArt }, null, 2)

const LENSES = [
  { key: 'architecture', q: 'Judge the ARCHITECTURE. Given the research on binding generators and multi-target transpilers: was dropping the C-ABI hub in favour of nanobind + node-addon-api correct? Is there a third option the prior art suggests that neither of us considered (WASM component model? uniffi-style codegen? something else)? Does the N x M bridge cost actually bite in practice for projects of this shape, or is it a theoretical worry? Should the IR or the bridge trait change shape at all? Also judge the TypeScript `number` question against how AssemblyScript/Porffor/Static Hermes handle it — what SHOULD compylr do, concretely?' },
  { key: 'risk', q: 'Judge the RISK. The audit found claims in this repository that were not true, in more than one place. What does the full set of surviving findings say about where else that pattern is likely to exist, and about what process change would catch it? Rank every surviving finding by how much damage it does to trust in the project, and say which must be fixed before the C++ work starts at all. Be specific about whether the (typescript, go) pair should be considered shipped, in-progress, or broken.' },
  { key: 'plan', q: 'Judge THE PLAN — openspec/changes/add-cpp-backend/ (PR #36). Read all four artifacts (proposal.md, design.md, tasks.md, specs/**) from disk yourself. Given everything in the dossier, produce a concrete revision list: which requirements, decisions, and tasks must be deleted, rewritten, or added. Cover at minimum: removing cpp-abi-bridge; re-specifying the two bridges on nanobind and node-addon-api; the C++26 targeting decision in light of the compiler-support research; the demo parity requirement now that the demo-ts-go standard is known to be fabricated; and whether the prerequisite fixes (#37, #38, #39, #40) belong in this change or in separate ones. Be concrete enough that someone could act on your list without re-deriving it.' },
]

// --- Segment 4: synthesis and critique, on Opus.
let judged = []
let merged = null
let critique = null
if (!await guard('before-synthesis')) {
  phase('Synthesize')
  judged = (await parallel(LENSES.map(l => () => agent([
    RULES, '', CONTEXT, '',
    '=== YOUR ASSIGNMENT: synthesis, lens "' + l.key + '" ===', '',
    l.q, '',
    'You have the full dossier below: adversarially-verified audit findings, adversarially-deepened research,',
    'and prior-art analysis. Read the repository yourself where you need to check something — do not take the',
    'dossier on faith, and say so where you disagree with it.', '',
    'Give a decisive recommendation, not a survey of options. Where you are uncertain, say what measurement',
    'or experiment would settle it. Write your full analysis to research/synthesis-' + l.key + '.md and return it.', '',
    'DOSSIER:', dossier,
  ].join('\n'), { label: 'synth:' + l.key, phase: 'Synthesize', model: 'opus', effort: 'xhigh' })))).filter(Boolean)

  merged = await agent([
    RULES, '', CONTEXT, '',
    '=== YOUR ASSIGNMENT: merge the three syntheses into one decision document ===', '',
    'Three independent analysts examined the same dossier through an architecture lens, a risk lens, and a',
    'plan-revision lens. Merge them into ONE document for the project owner.', '',
    'Where they agree, state it once. Where they DISAGREE, say so explicitly and adjudicate — do not average',
    'them into mush. Where all three missed something you can see in the dossier, add it.', '',
    'Structure the output as:',
    '1. What is actually broken, ranked, with file:line evidence, separating confirmed from unverified.',
    '2. What the research settles — the empirical questions that now have answers, with numbers and sources.',
    '3. What the research does NOT settle, and the cheapest experiment for each.',
    '4. Prior art worth acting on: what to steal, from where, and what to vendor.',
    '5. The concrete revision list for openspec/changes/add-cpp-backend/ — per artifact, per requirement.',
    '6. What should be a SEPARATE openspec change rather than part of this one, and why.', '',
    'Write it to research/DECISION.md and return the full text.', '',
    'THREE SYNTHESES:', JSON.stringify(judged, null, 2), '', 'DOSSIER:', dossier,
  ].join('\n'), { label: 'merge', phase: 'Synthesize', model: 'opus', effort: 'xhigh' })

  phase('Critique')
  critique = await agent([
    RULES, '', CONTEXT, '',
    '=== YOUR ASSIGNMENT: completeness critic ===', '',
    'Below is a decision document produced from an 8-dimension repo audit, 10 research topics, and a',
    'prior-art sweep. Your job is to find what the whole exercise MISSED.', '',
    '- Which parts of the repository did nobody look at? Enumerate the crates and directories and check',
    '  each against the audit coverage_notes. Name the blind spots.',
    '- Which claims in the decision document are asserted without evidence, or rest on a single unverified source?',
    '- Which research questions were answered thinly or not at all?',
    '- What would an expert in compiler design, or in Python/C++ interop specifically, say is obviously missing?',
    '- Is there a defect class nobody hunted for? The audit looked for "claims that are not true" —',
    '  what other class should have been searched?', '',
    'Verify at least three of the decision document\'s load-bearing claims yourself against the repository',
    'or the web, and report whether they hold.', '',
    'Write your critique to research/CRITIQUE.md. Return a prioritized list of concrete follow-up work,',
    'each item phrased so it could be handed to another agent as a task.', '',
    'DECISION DOCUMENT:', merged,
  ].join('\n'), { label: 'critique', phase: 'Critique', model: 'opus', effort: 'xhigh' })
}

return {
  paused,
  pause_info: pauseInfo,
  resume_hint: paused
    ? (pauseInfo.reason === 'over-threshold'
        ? 'Session limit reached ' + pauseInfo.utilization + '% (>= ' + pauseInfo.threshold + '%) at checkpoint "'
          + pauseInfo.at + '". Resume after ' + (pauseInfo.resets_at_iso || 'the window resets') + '.'
        : 'Stopped at checkpoint "' + pauseInfo.at + '" because ' + pauseInfo.reason + ' (' + pauseInfo.detail
          + '). No utilization reading was obtained, so the reset time is unknown — check the live bar at '
          + 'claude.ai/api/organizations/{uuid}/usage before resuming.')
      + ' Re-invoke Workflow with scriptPath "workflows/cpp-review.workflow.js" and resumeFromRunId '
      + 'to continue — completed agents replay from cache.'
    : null,
  audit_survived: totalSurvived,
  audit_refuted: totalKilled,
  audit_unverified: totalUnverified,
  audit: auditSummary,
  research: research.map(r => r && ({ topic: r.topic, summary: r.summary, implications: r.implications_for_compylr, file: r.context_file })).filter(Boolean),
  prior_art_ranked: priorArt.ranked.map(p => ({ name: p.name, url: p.repo_url, score: p.relevance_score })),
  prior_art_captured: priorArt.captured,
  decision: merged,
  critique,
}
