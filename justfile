set dotenv-load := true

fmt:
    cargo fmt --check
    cd apps/web && npm run format:check

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cd apps/web && npm run lint

test:
    cargo test --workspace
    cd apps/web && npm test

typecheck:
    cd apps/web && npm run typecheck

build:
    cargo build --workspace
    cd apps/web && npm run build

docs-pdf:
    cd apps/web && npm run docs:pdf

api:
    cargo run -p rag-debugger-api

web:
    cd apps/web && npm run dev

db-up:
    docker compose up -d postgres

db-down:
    docker compose down

db-migrate:
    sqlx migrate run

check: fmt lint typecheck test build

full-check: fmt lint typecheck test build
    cd apps/web && npm run size:check
    cd apps/web && npx playwright test
    cd apps/web && npm run docs:pdf
    docker compose up -d postgres
    sqlx migrate run
