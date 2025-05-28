.DEFAULT_GOAL := help

# Disallow warnings
RUST_LOG ?= info
RUSTFLAGS ?= -D warnings

# === Core Actions ===

.PHONY: fmt
fmt:
	@echo "Checking Rust formatting..."
	@cargo fmt --all -- --check

.PHONY: lint
lint:
	@echo "Running Rust linter..."
	@export RUSTFLAGS="$(RUSTFLAGS)"; cargo clippy --all-targets

.PHONY: fmt-apply
fmt-apply:
	@echo "Applying Rust formatting..."
	@cargo fmt --all

.PHONY: lint-apply
lint-apply:
	@echo "Applying Rust linter suggestions..."
	@cargo clippy --fix --allow-dirty

.PHONY: check
check:
	@echo "Checking compilation..."
	@export RUSTFLAGS="$(RUSTFLAGS)"; cargo check

.PHONY: test
test:
	@echo "Running tests..."
	@cargo test

.PHONY: build
build:
	@echo "Building project..."
	@cargo build

.PHONY: doc
doc:
	@echo "Building documentation..."
	@cargo doc --no-deps

.PHONY: run
run:
	@echo "Running project with cargo watch..."
	cargo watch \
		-E RUSTFLAGS="$(RUSTFLAGS)" \
		-E RUST_LOG="$(RUST_LOG)" \
		-w src \
		-x run

# === Aggregate Commands ===

.PHONY: verify
verify: fmt lint build test doc
	@echo "Verification passed."

.PHONY: fix
fix: fmt-apply lint-apply
	@echo "Auto-fixes applied."

# === Help ===

.PHONY: help
help:
	@echo "Usage: make [command]"
	@echo ""
	@echo "Core Actions:"
	@echo "  fmt              Check formatting with cargo fmt"
	@echo "  lint             Run clippy linter"
	@echo "  fmt-apply        Apply formatting changes"
	@echo "  lint-apply       Apply linter suggestions"
	@echo "  check            Fast compilation check"
	@echo "  build            Build the project"
	@echo "  test             Run tests"
	@echo "  doc              Generate documentation"
	@echo "  run              Run with cargo watch for development"
	@echo ""
	@echo "Aggregate Commands:"
	@echo "  verify           Run fmt, lint, build, test, doc checks"
	@echo "  fix              Apply fmt and lint fixes"
	@echo ""
	@echo "Other:"
	@echo "  help             Show this help message"