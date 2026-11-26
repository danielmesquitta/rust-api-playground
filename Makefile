.PHONY: all install build release run test test-unit test-integration db-up db-down fmt fmt-check lint doc msrv print-msrv deny machete actionlint fix watch-fix check clean update help

# Minimum supported Rust version, read from Cargo.toml so it is declared in one place.
MSRV := $(shell sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
ACTIONLINT_IMAGE := rhysd/actionlint:1.7.12

all: check

install: ## Install dev tools (watchexec, cargo-deny, cargo-machete)
	cargo install --locked watchexec-cli cargo-deny cargo-machete

build: ## Build in debug mode
	cargo build

release: ## Build optimized binary
	cargo build --release

run: ## Run the API server (loads .env if present)
	set -a; [ -f .env ] && . ./.env; set +a; cargo run

test: ## Run all tests (integration tests need Docker)
	cargo test --locked

test-unit: ## Run unit tests only (no Docker needed)
	cargo test --locked --lib

test-integration: ## Run integration tests against Postgres in Docker
	cargo test --locked --test api

db-up: ## Start the local Postgres and wait until it is healthy
	docker compose up -d --wait postgres

db-down: ## Stop the local Postgres (keeps its data volume)
	docker compose down

fmt: ## Format code
	cargo fmt

fmt-check: ## Check formatting without changing files
	cargo fmt -- --check

lint: ## Run clippy, treating warnings as errors
	cargo clippy --locked --all-targets --all-features -- -D warnings

doc: ## Build docs (warnings are errors) and run doc-tests
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --document-private-items
	cargo test --locked --doc

msrv: ## Check the code builds on the minimum supported Rust version
	cargo +$(MSRV) check --locked --all-targets

print-msrv: ## Print the minimum supported Rust version
	@echo $(MSRV)

deny: ## Check advisories, licenses, bans and sources (needs cargo-deny)
	cargo deny --locked check

machete: ## Find unused dependencies (needs cargo-machete)
	cargo machete

actionlint: ## Lint the GitHub Actions workflows (needs Docker)
	docker run --rm -v "$(CURDIR):/repo" -w /repo $(ACTIONLINT_IMAGE) -color

fix: ## Auto-apply clippy and compiler suggestions
	cargo clippy --fix --allow-dirty --allow-staged

watch-fix: ## Auto-apply clippy fixes whenever a .rs file changes
	watchexec -e rs -- cargo clippy --fix --allow-dirty --allow-staged

check: fmt-check lint doc deny machete test ## Everything CI runs, except msrv and actionlint

clean: ## Remove build artifacts
	cargo clean

update: ## Update agent skills and dependencies
	npx skills update -y && cargo update

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
