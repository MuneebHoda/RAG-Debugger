# Development Guide

## Prerequisites

- Rust via `rustup`
- Node.js 24 or newer
- Postgres for persistent features
- Docker Desktop or another Docker daemon for local Postgres
- SQLx CLI for manual migration commands: `cargo install sqlx-cli --no-default-features --features rustls,postgres`
- `just` is optional but recommended

## Setup

```sh
cp .env.example .env
cd apps/web && npm install
docker compose up -d postgres
```

## Run

API:

```sh
cargo run -p rag-debugger-api
```

The API connects to `DATABASE_URL`, runs migrations from `migrations/`, and creates a default local project on startup.

On startup the API also bootstraps the local demo identity from `.env`:

```text
RAG_DEBUGGER_BOOTSTRAP_EMAIL=demo@corpuslab.ai
RAG_DEBUGGER_BOOTSTRAP_PASSWORD=CorpusLab#2026
```

Use those credentials on `/login`, or create another local workspace through `/signup`.

Web:

```sh
cd apps/web && npm run dev
```

## Commands

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/web && npm run typecheck && npm run lint && npm test && npm run build
```

With `just`:

```sh
just db-up
just db-migrate
just rust-check
just web-check
just check
just ci-check
just full-check
just docs-pdf
just api
just web
```

- `rust-check`: Rust formatting, clippy, workspace tests, and workspace build.
- `web-check`: web formatting, typecheck, lint, tests, and production build.
- `check`: the fast local gate combining `rust-check` and `web-check`.
- `ci-check`: the release gate adding bundle budgets, Playwright, handbook generation, Postgres, and migrations.
- `full-check`: backward-compatible alias for `ci-check`.

## Database Flow

Persistent features should add SQLx migrations before repository implementations:

```sh
sqlx migrate add <change_name>
sqlx migrate run
```

The API runs migrations automatically at startup for local development. `just db-migrate` exists for explicit migration checks and CI-style workflows. `/readyz` checks database connectivity.

The `just` migration recipes honor `DATABASE_URL` from the environment or `.env`. When neither is present, they use the documented Docker default `postgres://postgres:postgres@localhost:5432/rag_debugger`.

## File Ingestion Flow

1. Start Postgres with `docker compose up -d postgres`.
2. Run the API with `cargo run -p rag-debugger-api`.
3. Run the web app with `cd apps/web && npm run dev`.
4. Open `http://127.0.0.1:5173/app/sources`.
5. Upload `.txt`, `.md`, `.markdown`, `.html`, `.htm`, or embedded-text `.pdf` files.

The default strategy is `structured`, which is tuned for general corpora: technical docs, policies, support KBs, research papers, code docs, contracts, wikis, and resumes. The API still accepts `chunking_strategy=smart_sections` as a legacy alias. Use the Sources page strategy selector or multipart `chunking_strategy=whitespace` to compare against plain whitespace windows.

Original uploaded binaries are not stored. The API stores source metadata, document profile, extraction quality, warnings, extracted chunk text, byte ranges, token counts, checksums, chunking strategy, section title, split reason, quality flags, duplicate status, text density, and evidence hints in Postgres.

When chunking behavior changes, add a migration for any persisted metadata and keep old rows readable with explicit defaults.

Audit report snapshots use the `debug_reports` table. List and detail repository methods require a workspace ID, and the canonical report body is stored as JSON with indexed ownership/source metadata. Run migrations before testing report persistence against Postgres.

Report APIs require the same HttpOnly session used by the workbench. Generate reports from saved traces, experiments, or CI eval runs; omitted privacy mode defaults to `metadata_only`. Full-local reports remain readable in the workbench but are rejected by the Markdown export endpoint.

## Retrieval Playground Flow

1. Complete the file ingestion flow so chunks exist in Postgres.
2. Open `http://127.0.0.1:5173/app/retrieval`.
3. Click `Index` in the Embeddings panel to create local chunk embeddings.
4. Ask a question and choose `hybrid`, `vector`, or `lexical` mode.
5. Inspect the evidence summary, citations, matched terms, semantic score, lexical score, normalized score bars, quality flags, evidence strength, and duplicate counts.
6. Save a good cited result to Eval Lab, then run an experiment after retrieval or chunking changes.

`POST /api/v1/retrieval/query` is local-only in v1. It uses local embeddings plus lexical and metadata scoring, stores the playground run/hits, and does not call hosted embedding or generation models.

`GET /api/v1/embeddings/status` shows whether chunks are indexed for the current local embedding model. `POST /api/v1/embeddings/index` synchronously indexes chunks and can later become a background worker entry point.

Legacy retrieval evals are still stored through `/api/v1/retrieval/evals` and run through `/api/v1/retrieval/evals/run`. New work should use Eval Lab datasets and experiments.

## Eval Lab Flow

1. Create or select a dataset in `/app/evals`.
2. Add cases manually, or save expected evidence directly from `/app/retrieval` or `/app/traces`.
3. Choose one or more modes: `lexical`, `vector`, and `hybrid`.
4. Run an experiment.
5. Inspect recall@k, precision@k, MRR, citation coverage, latency p50/p95, deterministic failure labels, and the release gate result.

Eval Lab APIs live under `/api/v1/eval-lab`. Existing retrieval eval cases are backfilled into `Default retrieval dataset` by migration so older local data remains usable.

## CI Gate Flow

1. Sign in to `/app/settings`.
2. Create a `CI API Keys` key and copy the one-time `clab_...` secret.
3. Store the key in GitHub Actions as `CORPUSLAB_API_KEY`.
4. Use `docs/examples/github-actions-corpuslab-evals.yml` as the starting workflow.
5. POST to `/api/v1/eval-lab/ci/runs` with `fail_on_gate=true` to fail the CI job when the Eval Lab gate fails.

CI runs are saved as Eval Lab experiments and appear in `/app/evals`, Mission Control, and Reports. API key secrets are stored only as hashes.

## Auth And Workspace Flow

Local development uses the `local` auth provider. Login and signup create opaque HttpOnly session cookies, and workbench APIs require a valid session. CI endpoints require workspace-scoped API keys with the `ci_eval_runs` scope.

The auth boundary is intentionally provider-shaped: local Postgres-backed sessions are implemented now, while external identity/session validation can replace the provider later without rewriting workbench routes.

## Trace Debugger Flow

1. Run a retrieval query from `/app/retrieval`.
2. Click `Save trace` in the Evidence Summary panel.
3. Open `/app/traces`.
4. Select the saved trace to inspect query input, retrieval ranking, evidence summary, eval status, ranked chunks, citations, and failure labels.
5. Use the Rerun Lab to compare `lexical`, `vector`, and `hybrid` retrieval with a different `top_k`.

Trace APIs live under `/api/v1/traces`. The API stores the full retrieval response on retrieval runs, then saves trace timelines and rerun comparisons through the storage repository. In development, the memory store supports the same flow as Postgres.

## Configuration Flow

Backend defaults are loaded from `.env` into typed config in `apps/api/src/config.rs`. Shared config contracts live in `crates/core/src/config.rs`. Safe values are exposed to the UI through `GET /api/v1/config`; deployment-sensitive values such as `DATABASE_URL` remain server-only.

When adding a new tunable value:

1. Add the field to `crates/core/src/config.rs`.
2. Load and validate it in `apps/api/src/config.rs`.
3. Add an entry to `.env.example`.
4. Display it in Settings only if it is safe for users to see.

## Technical Handbook

The source handbook is `docs/technical-handbook.md`. Generate `docs/technical-handbook.pdf` with:

```sh
just docs-pdf
```

## Adding a Feature

1. Add or update domain types in `crates/core`.
2. Add behavior interfaces in `crates/rag` or repository traits in `crates/storage`.
3. Implement API handlers under `apps/api`.
4. Add thin UI route wrappers under `apps/web/src/pages` and implementation under `apps/web/src/features/workbench/<domain>`.
5. Add tests at the lowest useful layer.
6. Update docs or ADRs when the architecture changes.

Read `docs/frontend-architecture.md` before adding or reorganizing frontend code. Route wrappers should compose or re-export feature pages; server state and workflow actions belong in domain hooks when page orchestration becomes complex.
