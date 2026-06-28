set dotenv-load := true

database_url := env_var_or_default("DATABASE_URL", "postgres://postgres:postgres@localhost:5432/rag_debugger")

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
    DATABASE_URL='{{ database_url }}' sqlx migrate run

rust-check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo build --workspace

web-check:
    cd apps/web && npm run format:check
    cd apps/web && npm run typecheck
    cd apps/web && npm run lint
    cd apps/web && npm test -- --run
    cd apps/web && npm run build

ci-check: rust-check web-check
    cd apps/web && npm run size:check
    cd apps/web && npx playwright test
    cd apps/web && npm run docs:pdf
    docker compose up -d postgres
    DATABASE_URL='{{ database_url }}' sqlx migrate run

check: rust-check web-check

full-check: ci-check
