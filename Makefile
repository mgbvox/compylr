# compylr — the commands worth having a short name for.
#
# Nothing here is load-bearing: every target is a line you could type yourself, and the README and
# CLAUDE.md still show the underlying commands. What the file buys is the handful of details that
# are easy to get wrong and expensive to discover — that coverage must run with the venv
# deactivated, that the binary lives in `compylr-cli` now, that a backend change needs `.compylr`
# removed because the rebuild key is the IR fingerprint and the compiler's version does not move
# during development.
#
# `make help` lists everything.

.DEFAULT_GOAL := help
SHELL := /bin/bash

VENV := .venv
PY := $(VENV)/bin/python
DEMO := demo
FIXTURE ?= python/fixtures/accepted/inference.py
N ?= 500
SCALE ?= 1

# Coverage runs `cargo test`, and the bridge tests auto-initialize a Python interpreter. An active
# venv makes that mismatch what PyO3 linked against and the suite aborts with "no Python frame",
# which looks like a real failure and is not.
NO_VENV := env -u VIRTUAL_ENV -u PYTHONHOME

.PHONY: help
help: ## List the available targets
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# -- everyday ---------------------------------------------------------------------------

.PHONY: check
check: fmt-check lint doc test python ## Everything CI runs: format, lint, docs, both suites

.PHONY: test
test: ## Run the Rust suite
	cargo test --workspace

.PHONY: lint
lint: ## Clippy, warnings denied
	cargo clippy --workspace --all-targets -- -D warnings

.PHONY: fmt
fmt: ## Format every crate
	cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Fail if anything is unformatted
	cargo fmt --all --check

.PHONY: doc
doc: ## Build the docs, warnings denied
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --lib

.PHONY: python
python: develop py-lint py-types ## Run the Python suite, its linters, and its type checker
	$(PY) -m pytest

.PHONY: py-lint
py-lint: $(VENV) ## Ruff, both halves: the lint rules and the formatter
	$(VENV)/bin/ruff check python/ scripts/
	$(VENV)/bin/ruff format --check python/ scripts/

.PHONY: py-fmt
py-fmt: $(VENV) ## Format the Python sources
	$(VENV)/bin/ruff format python/ scripts/

.PHONY: py-types
py-types: $(VENV) ## Type-check the package with ty
	$(VENV)/bin/ty check python/compylr

.PHONY: hooks
hooks: $(VENV) ## Install the pre-commit hooks, once per clone
	$(VENV)/bin/pre-commit install

.PHONY: precommit
precommit: $(VENV) ## Run every pre-commit hook over the whole tree
	$(VENV)/bin/pre-commit run --all-files

# Depends on clean-artifacts deliberately: the rebuild key is the IR fingerprint and the compiler's
# version does not move during development, so a cached build would have this timing last build's
# code and reporting it as this one's.
.PHONY: benchmarks
benchmarks: clean-artifacts develop ## Re-measure and rewrite the README tables: make benchmarks SCALE=4
	$(PY) scripts/update_benchmarks.py --scale $(SCALE) --n $(N)

.PHONY: coverage
coverage: ## Rust coverage, with the venv out of the way
	$(NO_VENV) cargo llvm-cov --workspace \
		--ignore-filename-regex '(vendored/|/main\.rs)' --summary-only

# -- building ---------------------------------------------------------------------------

$(VENV):
	uv venv
	uv pip install --python $(PY) maturin pytest pytest-cov ruff ty pre-commit

.PHONY: develop
develop: $(VENV) ## Rebuild compylr._core into the venv
	source $(VENV)/bin/activate && maturin develop --release

.PHONY: clean-artifacts
clean-artifacts: ## Drop generated crates, so a backend change is picked up
	rm -rf .compylr $(DEMO)/.compylr

.PHONY: clean
clean: clean-artifacts ## Also drop Rust build output
	cargo clean

# -- the compiler, by hand --------------------------------------------------------------

.PHONY: run
run: ## Summarise a fixture: make run FIXTURE=path/to.py
	cargo run -p compylr-cli -- $(FIXTURE)

.PHONY: emit-ir
emit-ir: ## Print a fixture's IR as JSON
	cargo run -p compylr-cli -- --emit ir $(FIXTURE)

.PHONY: emit-rust
emit-rust: ## Print a fixture's translated Rust
	cargo run -p compylr-cli -- --emit rust $(FIXTURE)

# -- the demo ---------------------------------------------------------------------------

.PHONY: demo
demo: develop ## Every algorithm, compiled against interpreted: make demo SCALE=4
	cd $(DEMO) && uv sync --extra dev --quiet
	cd $(DEMO) && uv run python -m algorithms.benchmark --scale $(SCALE)

.PHONY: demo-performance
demo-performance: develop ## Slow: guard recorded scale-four speedups against measured noise
	cd $(DEMO) && uv sync --extra dev --quiet
	cd $(DEMO) && uv run python -m algorithms.benchmark --scale 4 --check-performance

.PHONY: demo-primes
demo-primes: develop ## The nth prime three ways, compiled against interpreted: make demo-primes N=500
	cd $(DEMO) && uv sync --extra dev --quiet
	cd $(DEMO) && uv run python -m algorithms.nth_prime.benchmark --n $(N)

.PHONY: demo-run
demo-run: develop ## Run every algorithm and print the IR coverage table
	cd $(DEMO) && uv sync --quiet && uv run compylr compyle src
	cd $(DEMO) && uv run python -m algorithms

.PHONY: demo-primes-run
demo-primes-run: develop ## Run the three nth-prime variants: make demo-primes-run N=25
	cd $(DEMO) && uv sync --quiet && uv run compylr compyle src
	cd $(DEMO) && uv run python -m algorithms.nth_prime $(N)

# `ty check src` and not `src tests`: a `@compyle`-marked class is a *value* -- the decorator
# returns a wrapper, not a class -- so the marked name cannot be used as a type annotation, and
# `tests/test_variants.py` annotates a fixture with `memoized.PrimeCache`. Widening this means
# answering what a marked class should look like to a type checker, which is a change to compylr's
# public typing surface rather than to CI.
.PHONY: demo-check
demo-check: develop ## The demo's own suite and linters
	cd $(DEMO) && uv sync --extra dev --quiet
	cd $(DEMO) && uv run compylr compyle src
	cd $(DEMO) && uv run pytest
	cd $(DEMO) && uv run ruff check .
	cd $(DEMO) && uv run ruff format --check .
	cd $(DEMO) && uv run ty check src

.PHONY: demo-rebuild
demo-rebuild: clean-artifacts demo-check ## Rebuild the demo from nothing, then check it

# -- specs ------------------------------------------------------------------------------

.PHONY: spec
spec: ## Validate every spec and active change
	openspec validate --specs --strict
	openspec list
