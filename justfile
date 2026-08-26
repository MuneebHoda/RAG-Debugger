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

governance-check:
    cd apps/web && npm run governance:check

rust-check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo build --workspace

web-check:
    cd apps/web && npm run format:check
    cd apps/web && npm run typecheck
    cd apps/web && npm run lint
    cd apps/web && npm run test:unit -- --run
    cd apps/web && npm run build

ci-check: governance-check rust-check web-check
    cd apps/web && npm run size:check
    cd apps/web && npx playwright test
    cd apps/web && npm run docs:pdf
    docker compose up -d postgres
    DATABASE_URL='{{ database_url }}' sqlx migrate run
    DATABASE_URL='{{ database_url }}' cargo test -p rag-debugger-storage --test evidence_repository_contract postgres_evidence_repository_is_deterministic_and_bounded -- --ignored
    DATABASE_URL='{{ database_url }}' cargo test -p rag-debugger-storage --test eval_workspace_contract postgres_eval_repository_enforces_workspace_ownership -- --ignored
    DATABASE_URL='{{ database_url }}' cargo test -p rag-debugger-storage --test eval_workspace_contract postgres_eval_corpus_snapshot_stays_consistent_across_concurrent_mutation -- --ignored
    DATABASE_URL='{{ database_url }}' cargo test -p rag-debugger-storage --test runtime_workspace_contract postgres_runtime_repository_enforces_workspace_ownership -- --ignored
    DATABASE_URL='{{ database_url }}' cargo test -p rag-debugger-storage --test workspace_migration workspace_ownership_migration_backfills_singletons_and_quarantines_ambiguity -- --ignored
    DATABASE_URL='{{ database_url }}' cargo test -p rag-debugger-storage postgres_evidence_query_plans_are_index_compatible -- --ignored
    DATABASE_URL='{{ database_url }}' cargo test -p rag-debugger-storage --test trace_ingestion_repository_contract postgres_trace_ingestion_repository_contract -- --ignored

trace-ingestion-smoke:
    cargo test -p rag-debugger-api --test trace_ingestion trace_key_authorizes_protobuf_otlp_and_invalid_bearer_never_uses_session

check: governance-check rust-check web-check

full-check: ci-check
