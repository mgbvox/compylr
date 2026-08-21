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
check: fmt lint test python ## Everything CI would run: format, lint, both suites

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

.PHONY: python
python: develop ## Run the Python suite and its linters
	$(PY) -m pytest
	$(VENV)/bin/ruff check python/
	$(VENV)/bin/mypy python/compylr

.PHONY: coverage
coverage: ## Rust coverage, with the venv out of the way
	$(NO_VENV) cargo llvm-cov --workspace \
		--ignore-filename-regex '(vendored/|/main\.rs)' --summary-only

# -- building ---------------------------------------------------------------------------

$(VENV):
	uv venv
	uv pip install --python $(PY) maturin pytest pytest-cov ruff mypy

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

.PHONY: demo-check
demo-check: develop ## The demo's own suite and linters
	cd $(DEMO) && uv sync --extra dev --quiet
	cd $(DEMO) && uv run compylr compyle src
	cd $(DEMO) && uv run pytest
	cd $(DEMO) && uv run ruff check .
	cd $(DEMO) && uv run ruff format --check .
	cd $(DEMO) && uv run mypy src

.PHONY: demo-rebuild
demo-rebuild: clean-artifacts demo-check ## Rebuild the demo from nothing, then check it

# -- specs ------------------------------------------------------------------------------

.PHONY: spec
spec: ## Validate every spec and active change
	openspec validate --specs --strict
	openspec list
