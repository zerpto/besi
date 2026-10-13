.DEFAULT_GOAL := help
.PHONY: help build test lint fmt doc check clean

help: ## Show available targets
	@grep -E '^[a-z]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-8s %s\n", $$1, $$2}'

build: ## Build with every feature
	cargo build --all-features

test: ## Run all tests with every feature
	cargo test --all-features

lint: ## Clippy with warnings as errors, for each feature alone, none, and all
	cargo clippy --all-targets --all-features -- -D warnings
	cargo clippy --all-targets --no-default-features -- -D warnings
	cargo clippy --all-targets --no-default-features --features api -- -D warnings
	cargo clippy --all-targets --no-default-features --features db -- -D warnings
	cargo clippy --all-targets --no-default-features --features cron -- -D warnings
	cargo clippy --all-targets --no-default-features --features orm -- -D warnings

fmt: ## Format all code
	cargo fmt

doc: ## Build API docs with every feature and open them
	cargo doc --all-features --no-deps --open

check: ## Format check, lint, and test (CI gate)
	cargo fmt --check
	$(MAKE) lint test

clean: ## Remove build output
	cargo clean
