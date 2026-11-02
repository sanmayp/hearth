# Hearth Makefile. Short verbs only.

.PHONY: g0 g1  build fmt clippy test ui-dev
SHELL := bash
export PATH := $(HOME)/.cargo/bin:$(PATH)

g0: ## Lane gate (<60s)
	bash scripts/g0.sh

g1: ## Merge train (<4min)
	bash scripts/g1.sh

build: ## Release build (server + UI)
	cargo build --release --workspace
	cd web && npm run build

fmt: ## Format Rust
	cargo fmt

clippy: ## Lint Rust (deny warnings)
	cargo clippy --workspace --all-targets -- -D warnings

test: ## All Rust tests
	cargo test --workspace 2>/dev/null || cargo nextest run --workspace

ui-dev: ## Run UI dev server
	cd web && npm run dev
