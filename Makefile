# Binary name, read from [[bin]] in crates/app/Cargo.toml (single source of truth).
BIN  := $(shell sed -n '/^\[\[bin\]\]/,/^name/s/^name *= *"\(.*\)"/\1/p' crates/app/Cargo.toml)
DIST := dist

.DEFAULT_GOAL := help
.PHONY: help dev prod run test lint fmt check clean

help: ## Show available targets
	@grep -E '^[a-z]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-8s %s\n", $$1, $$2}'

dev: ## Build debug binary -> dist/dev/
	cargo build -p app
	@mkdir -p $(DIST)/dev
	@cp target/debug/$(BIN) $(DIST)/dev/$(BIN)
	@echo "built $(DIST)/dev/$(BIN)"

prod: ## Build optimized release binary -> dist/prod/
	cargo build -p app --release
	@mkdir -p $(DIST)/prod
	@cp target/release/$(BIN) $(DIST)/prod/$(BIN)
	@echo "built $(DIST)/prod/$(BIN)"

run: ## Run via cargo, e.g. make run ARGS="hello Ferris"
	cargo run -p app -- $(ARGS)

test: ## Run all tests
	cargo test --workspace

lint: ## Clippy with warnings as errors (core checked with each feature combination)
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy -p besi --all-targets --no-default-features -- -D warnings
	cargo clippy -p besi --all-targets --no-default-features --features api -- -D warnings
	cargo clippy -p besi --all-targets --no-default-features --features db -- -D warnings
	cargo clippy -p besi --all-targets --no-default-features --features cron -- -D warnings
	cargo clippy -p besi --all-targets --no-default-features --features orm -- -D warnings

fmt: ## Format all code
	cargo fmt --all

check: ## Format check, lint, and test (CI gate)
	cargo fmt --all --check
	$(MAKE) lint test

clean: ## Remove build output
	cargo clean
	rm -rf $(DIST)
