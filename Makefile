SHELL := /bin/sh

CARGO ?= cargo
MSRV ?= 1.85.0

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show available targets
	@awk 'BEGIN {FS = ":.*## "; printf "Available targets:\n"} /^[a-zA-Z0-9_.-]+:.*## / {printf "  %-10s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: check
check: ## Run cargo check for all targets
	$(CARGO) check --all-targets

.PHONY: build
build: ## Build the crate
	$(CARGO) build

.PHONY: test
test: ## Run unit and integration tests
	$(CARGO) test
	$(CARGO) test --doc

.PHONY: fmt
fmt: ## Format the codebase
	$(CARGO) fmt

.PHONY: lint
lint: ## Run formatting and clippy checks
	$(CARGO) fmt --check
	$(CARGO) clippy -- --deny warnings

.PHONY: doc
doc: ## Build local API documentation
	$(CARGO) doc --no-deps

.PHONY: ci
ci: build test lint doc ## Run the full local verification suite

.PHONY: msrv
msrv: ## Run build and test with the minimum supported Rust version
	$(CARGO) +$(MSRV) build
	$(CARGO) +$(MSRV) test

.PHONY: clean
clean: ## Remove build artifacts
	$(CARGO) clean
