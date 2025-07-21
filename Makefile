# Big Bot Development Makefile
.PHONY: help build test clean install dev fmt clippy audit release docker docs bench

# Default target
help: ## Show this help message
	@echo "Big Bot Development Commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Examples:"
	@echo "  make dev          # Start development server with hot reload"
	@echo "  make test         # Run all tests"
	@echo "  make release      # Build optimized release binary"

# Development
dev: ## Start development server with hot reload
	cargo watch -x 'run -- --verbose'

build: ## Build the project in debug mode
	cargo build

build-release: ## Build the project in release mode
	cargo build --release

# Testing
test: ## Run all tests
	cargo test

test-verbose: ## Run tests with verbose output
	cargo test -- --nocapture

test-coverage: ## Generate test coverage report
	cargo llvm-cov --html --open

test-integration: ## Run integration tests only
	cargo test --test integration

bench: ## Run benchmarks
	cargo bench

# Code Quality
fmt: ## Format code using rustfmt
	cargo fmt --all

fmt-check: ## Check if code is properly formatted
	cargo fmt --all -- --check

clippy: ## Run clippy linter
	cargo clippy --all-targets --all-features -- -D warnings

clippy-fix: ## Auto-fix clippy suggestions
	cargo clippy --fix --all-targets --all-features

audit: ## Security audit of dependencies
	cargo audit

# Cleaning
clean: ## Clean build artifacts
	cargo clean
	rm -rf target/
	rm -rf ~/.bigbot/database.sql*  # Clean test database

clean-all: clean ## Clean everything including caches
	rm -rf ~/.cargo/registry/cache/
	rm -rf ~/.cargo/git/

# Installation and setup
install-deps: ## Install development dependencies
	cargo install cargo-watch
	cargo install cargo-edit
	cargo install cargo-audit
	cargo install cargo-llvm-cov
	cargo install cargo-criterion

setup: install-deps ## Full development setup
	@echo "Setting up development environment..."
	rustup component add rustfmt clippy
	@echo "Development environment ready!"

# Release
release: ## Build optimized release binary
	cargo build --release --locked
	@echo "Release binary available at: target/release/bigbot"

install: release ## Install to system PATH
	sudo cp target/release/bigbot /usr/local/bin/
	@echo "Installed to /usr/local/bin/bigbot"

# Docker
docker-build: ## Build Docker image
	docker build -t big-bot:dev .

docker-run: docker-build ## Build and run Docker container
	docker run --rm -it \
		-v $(PWD)/data:/app/.bigbot \
		big-bot:dev

docker-compose-up: ## Start with docker-compose
	docker-compose up -d

docker-compose-down: ## Stop docker-compose services
	docker-compose down

docker-compose-logs: ## View docker-compose logs
	docker-compose logs -f

# Documentation
docs: ## Generate and open documentation
	cargo doc --open --no-deps

docs-all: ## Generate documentation including dependencies
	cargo doc --open

# Database management
db-reset: ## Reset development database
	rm -f ~/.bigbot/database.sql*
	@echo "Database reset. Will be recreated on next run."

# Configuration
config-example: ## Generate example configuration
	mkdir -p ~/.bigbot
	cargo run -- --help > /dev/null 2>&1 || true
	@echo "Example configuration created at ~/.bigbot/config.yml"

# Performance and profiling
profile: ## Profile the application (requires perf)
	cargo build --release
	perf record --call-graph=dwarf ./target/release/bigbot --help
	perf report

flamegraph: ## Generate flamegraph (requires cargo-flamegraph)
	cargo install flamegraph
	cargo flamegraph -- --help

# Maintenance
update: ## Update all dependencies
	cargo update

check-outdated: ## Check for outdated dependencies
	cargo install cargo-outdated
	cargo outdated

# CI/CD simulation
ci-test: ## Run the same tests as CI
	make fmt-check
	make clippy
	make test
	make audit

pre-commit: ci-test ## Run pre-commit checks
	@echo "All pre-commit checks passed!"

# Quick development workflows
quick-test: fmt clippy test ## Quick development cycle: format, lint, test

full-check: clean fmt clippy test audit docs ## Full check before committing

# Environment-specific builds
build-linux: ## Build for Linux (cross-compilation)
	cargo build --release --target x86_64-unknown-linux-gnu

build-windows: ## Build for Windows (cross-compilation)
	cargo build --release --target x86_64-pc-windows-gnu

build-macos: ## Build for macOS (cross-compilation)
	cargo build --release --target x86_64-apple-darwin

# Utility commands
size: ## Show binary size information
	@echo "Debug binary:"
	@ls -lh target/debug/bigbot 2>/dev/null || echo "Debug binary not built"
	@echo "Release binary:"
	@ls -lh target/release/bigbot 2>/dev/null || echo "Release binary not built"

deps-tree: ## Show dependency tree
	cargo tree

deps-licenses: ## Show dependency licenses
	cargo install cargo-license
	cargo license

# Development server setup (for testing)
mumble-server: ## Start local Mumble server (requires Docker)
	docker run -d --name test-mumble \
		-p 64738:64738 \
		-e MUMBLE_CONFIG_welcometext="Big Bot Test Server" \
		mumblevoip/mumble-server:latest

mumble-server-stop: ## Stop local Mumble server
	docker stop test-mumble && docker rm test-mumble

# Version management
version-patch: ## Bump patch version
	cargo install cargo-edit
	cargo set-version --bump patch

version-minor: ## Bump minor version
	cargo install cargo-edit
	cargo set-version --bump minor

version-major: ## Bump major version
	cargo install cargo-edit
	cargo set-version --bump major

# Help for specific areas
help-dev: ## Show development-specific help
	@echo "Development Workflow:"
	@echo "  1. make setup          # One-time setup"
	@echo "  2. make dev            # Start development server"
	@echo "  3. make quick-test     # Test your changes"
	@echo "  4. make pre-commit     # Before committing"

help-deploy: ## Show deployment help
	@echo "Deployment Options:"
	@echo "  1. make release        # Build release binary"
	@echo "  2. make docker-build   # Build Docker image"
	@echo "  3. make install        # Install to system"
