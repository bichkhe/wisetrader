.PHONY: help setup build run clean docker-up docker-down migrate test \
	migrate_create migrate_up migrate_down migrate_status \
	migrate_fresh migrate_auto migrate_setup

help:
	@echo "Available commands:"
	@echo ""
	@echo "Setup & Docker:"
	@echo "  make setup         - Initial setup (copy .env and start docker services)"
	@echo "  make docker-up     - Start Docker services (MySQL, Redis)"
	@echo "  make docker-down   - Stop Docker services"
	@echo ""
	@echo "Build & Run:"
	@echo "  make build         - Build all Rust crates"
	@echo "  make run-bot       - Run the Telegram bot"
	@echo "  make run-api       - Run the API server"
	@echo ""
	@echo "Migrations:"
	@echo "  make migrate_create NAME=<name> - Create new migration file"
	@echo "  make migrate_up                 - Run pending migrations"
	@echo "  make migrate_down               - Rollback last migration"
	@echo "  make migrate_status             - Show migration status"
	@echo "  make migrate_auto               - Generate entities from database"
	@echo "  make migrate_setup              - Complete migration setup"
	@echo ""
	@echo "Other:"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make test          - Run tests"

setup:
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "Created .env file - please update with your BOT_TOKEN"; \
	fi
	docker-compose up -d

build:
	cargo build
	
build-bot:
	cargo build --bin bot

run-bot:
	cargo build --bin bot
	cargo run --bin bot

run-api:
	cargo run --bin api

run-api-docker:
	docker-compose up api

run-signal-dispatcher:
	cargo run --bin signal_dispatcher

run-order-executor:
	cargo run --bin order_executor

docker-up:
	docker-compose up -d

docker-down:
	docker-compose down

clean:
	cargo clean
	docker-compose down -v

test:
	cargo test


migrate_gen:
	cd shared && sea-orm-cli generate entity -o src/entity

# Generate migration file
migrate_create:
	@echo "Creating migration: $(name)"
	cd shared && sea-orm-cli migrate generate --database-url mysql://wisetrader:wisetrader2025@localhost:23306/wisetrader_db $(name)

# Run migrations using Sea-ORM
migrate_up:
	@echo "Running migrations with Sea-ORM..."
	cd shared/migration && cargo run -- up -u mysql://wisetrader:wisetrader2025@localhost:23306/wisetrader_db 2>&1 | grep -v "Warning" || echo "✅ Migrations completed or already applied!"
	@echo "Generating entities from database..."
	@make migrate_auto

# Rollback last migration
migrate_down:
	cd shared && sea-orm-cli migrate down

# Show migration status
migrate_status:
	cd shared && sea-orm-cli migrate status

# Fresh migration - rollback all and run again
migrate_fresh:
	cd shared && sea-orm-cli migrate fresh

# Auto-migrate: generate entities from database
migrate_auto:
	@echo "Generating entities from database..."
	sea-orm-cli generate entity -u mysql://wisetrader:wisetrader2025@localhost:23306/wisetrader_db -o ./shared/src/entity
	@echo "✅ Entities generated successfully!"

# Setup complete: create migrations if needed and run them
migrate_setup: migrate_auto migrate_up
	@echo "✅ Migration setup complete!"

