.PHONY: help build run test clean docker-up docker-down migrate dev prod

# Variables
BINARY_NAME=treichville-exchange
DOCKER_IMAGE=treichville-exchange:latest

# Colors for output
RED=\033[0;31m
GREEN=\033[0;32m
YELLOW=\033[1;33m
NC=\033[0m # No Color

help: ## Show this help message
	@echo "$(GREEN)Treichville Exchange - Makefile Commands$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "$(YELLOW)%-20s$(NC) %s\n", $$1, $$2}'

build: ## Build the application
	@echo "$(GREEN)Building application...$(NC)"
	cargo build --release

run: ## Run the application
	@echo "$(GREEN)Running application...$(NC)"
	cargo run

dev: ## Run in development mode with auto-reload
	@echo "$(GREEN)Starting development server...$(NC)"
	cargo watch -x run

test: ## Run all tests
	@echo "$(GREEN)Running tests...$(NC)"
	cargo test -- --nocapture

test-unit: ## Run unit tests only
	@echo "$(GREEN)Running unit tests...$(NC)"
	cargo test --lib

test-integration: ## Run integration tests only
	@echo "$(GREEN)Running integration tests...$(NC)"
	cargo test --test '*'

lint: ## Run clippy linter
	@echo "$(GREEN)Running clippy...$(NC)"
	cargo clippy --all-targets --all-features -- -D warnings

fmt: ## Format code
	@echo "$(GREEN)Formatting code...$(NC)"
	cargo fmt

fmt-check: ## Check code formatting
	@echo "$(GREEN)Checking code format...$(NC)"
	cargo fmt -- --check

clean: ## Clean build artifacts
	@echo "$(RED)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf target/

docker-build: ## Build Docker image
	@echo "$(GREEN)Building Docker image...$(NC)"
	docker build -t $(DOCKER_IMAGE) .

docker-up: ## Start Docker services
	@echo "$(GREEN)Starting Docker services...$(NC)"
	docker-compose up -d

docker-down: ## Stop Docker services
	@echo "$(RED)Stopping Docker services...$(NC)"
	docker-compose down

docker-logs: ## Show Docker logs
	docker-compose logs -f

docker-clean: ## Clean Docker volumes
	@echo "$(RED)Cleaning Docker volumes...$(NC)"
	docker-compose down -v

migrate: ## Run database migrations
	@echo "$(GREEN)Running database migrations...$(NC)"
	sqlx migrate run

migrate-revert: ## Revert last migration
	@echo "$(YELLOW)Reverting last migration...$(NC)"
	sqlx migrate revert

db-reset: ## Reset database
	@echo "$(RED)Resetting database...$(NC)"
	sqlx database reset -y

setup: docker-up migrate ## Setup development environment
	@echo "$(GREEN)Development environment ready!$(NC)"
	@echo "API available at: http://localhost:8080"
	@echo "PostgreSQL available at: localhost:5432"
	@echo "Redis available at: localhost:6379"
	@echo "PgAdmin available at: http://localhost:5050"
	@echo "Redis Commander available at: http://localhost:8081"

prod: ## Build for production
	@echo "$(GREEN)Building for production...$(NC)"
	cargo build --release
	@echo "$(GREEN)Binary available at: target/release/$(BINARY_NAME)$(NC)"

bench: ## Run benchmarks
	@echo "$(GREEN)Running benchmarks...$(NC)"
	cargo bench

audit: ## Security audit
	@echo "$(GREEN)Running security audit...$(NC)"
	cargo audit

docs: ## Generate documentation
	@echo "$(GREEN)Generating documentation...$(NC)"
	cargo doc --no-deps --open

coverage: ## Generate test coverage
	@echo "$(GREEN)Generating test coverage...$(NC)"
	cargo tarpaulin --out Html

check: fmt-check lint test ## Run all checks
	@echo "$(GREEN)All checks passed!$(NC)"

install-tools: ## Install development tools
	@echo "$(GREEN)Installing development tools...$(NC)"
	cargo install cargo-watch
	cargo install cargo-audit
	cargo install cargo-tarpaulin
	cargo install sqlx-cli

health: ## Check service health
	@curl -f http://localhost:8080/health && echo "$(GREEN)\nAPI is healthy$(NC)" || echo "$(RED)\nAPI is not responding$(NC)"

logs: ## Show application logs
	@tail -f logs/app.log

.DEFAULT_GOAL := help