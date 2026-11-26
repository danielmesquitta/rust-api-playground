.PHONY: all install build release run test test-unit test-integration db-up db-down fmt fmt-check lint fix watch-fix check clean update help

all: check

install: ## Install dev tools (watchexec)
	cargo install watchexec-cli

build: ## Build in debug mode
	cargo build

release: ## Build optimized binary
	cargo build --release

run: ## Run the API server (loads .env if present)
	set -a; [ -f .env ] && . ./.env; set +a; cargo run

test: ## Run all tests (integration tests need Docker)
	cargo test

test-unit: ## Run unit tests only (no Docker needed)
	cargo test --lib

test-integration: ## Run integration tests against Postgres in Docker
	cargo test --test api

db-up: ## Start the local Postgres and wait until it is healthy
	docker compose up -d --wait postgres

db-down: ## Stop the local Postgres (keeps its data volume)
	docker compose down

fmt: ## Format code
	cargo fmt

fmt-check: ## Check formatting without changing files
	cargo fmt -- --check

lint: ## Run clippy, treating warnings as errors
	cargo clippy --all-targets --all-features -- -D warnings

fix: ## Auto-apply clippy and compiler suggestions
	cargo clippy --fix --allow-dirty --allow-staged

watch-fix: ## Auto-apply clippy fixes whenever a .rs file changes
	watchexec -e rs -- cargo clippy --fix --allow-dirty --allow-staged

check: fmt-check lint test ## Everything CI would run

clean: ## Remove build artifacts
	cargo clean

update: ## Update agent skills and dependencies
	npx skills update -y && cargo update

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
