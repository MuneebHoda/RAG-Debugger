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

api:
    cargo run -p rag-debugger-api

web:
    cd apps/web && npm run dev

check: fmt lint typecheck test build

