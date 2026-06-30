# Testing Guide

## Rust

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected coverage in the scaffold:

- API health/readiness smoke tests.
- Chunker behavior tests for structured headings, bullet grouping, oversized-block fallback, overlap, whitespace windows, and checksum stability.
- Document intelligence tests for profile detection, extraction quality, duplicate chunks, heading-only chunks, and evidence hints.
- File extraction, multipart ingestion, source listing, chunk listing, strategy metadata, and structured failure tests.
- Local embedding tests for deterministic dimensions, cosine similarity, related-domain matching, and dimension mismatch behavior.
- Local retrieval tests for token normalization, lexical scoring, vector scoring, hybrid missing-embedding behavior, phrase boosts, section/path boosts, deduplication, quality flags, evidence strength, insufficient evidence, and cited evidence summaries.
- Retrieval eval tests for recall@k, precision@k, MRR, citation coverage, top-hit rank, weak evidence counts, missing embedding failures, deterministic failure labels, and pass/fail calculation.
- Public regression fixtures for support knowledge bases, policy documents, and technical documentation, with expected retrieval, trace, and Eval Lab outcomes under `fixtures/`.
- API retrieval tests for all-doc search, document filtering, top-k, no-match response, embedding status/indexing, missing embeddings, lexical fallback mode, eval creation, eval run persistence, and request validation.
- Trace tests for trace construction, failure label assignment, rerun comparison, trace creation from retrieval runs, trace listing/detail, rerun API behavior, and missing-trace errors.
- Eval Lab API tests for dataset CRUD, case create/update/delete, legacy case backfill, cross-mode experiments, experiment comparison, gate evaluation, and failure diagnosis.
- Auth and workspace tests for signup, login, logout, current-user, session cookies, duplicate email behavior, membership role, and protected workbench routes.
- API key and CI eval tests for one-time secret generation, hashed storage, scoped authorization, revoke behavior, CI run persistence, gate failure status, and `fail_on_gate`.
- API error contract tests for structured 400, 401, 404, and sanitized internal/storage responses.
- MemoryStore contract coverage for health, project bootstrap, source/document/chunk persistence, chunk ordering, embedding candidates, and embedding status transitions.
- Domain serialization tests as contracts become public.
- Audit-report contract tests for source discriminators, privacy-mode wire values, optional evidence metadata, RFC3339 timestamps, and JSON round trips.

DB-backed integration checks require local Postgres:

```sh
docker compose up -d postgres
sqlx migrate run
```

Run the focused in-memory storage contract with:

```sh
cargo test -p rag-debugger-storage --test memory_store_contract
cargo test -p rag-debugger-rag --test public_fixtures
```

RAG behavior guarantees and fixture-change rules are defined in [`docs/rag-invariants.md`](rag-invariants.md). Engine tests should prefer typed Rust responses for precise contract coverage; public JSON fixtures remain small, synthetic, and readable for cross-language tooling.

## Web

Run:

```sh
cd apps/web
npm run typecheck
npm run lint
npm test
npm run build
npm run size:check
```

Expected coverage in the scaffold:

- App shell and workbench navigation render tests.
- Corpus render and focused document-detail tests, including the advanced structured chunking control.
- Test Retrieval render and mocked query tests, including one mode control, collapsed advanced settings, evidence summary, score bars, citations, and direct debugger navigation.
- Runs tests for search/list navigation, focused detail tabs, failure diagnosis, rerun comparison, and explicit dataset/evidence selection for Quality.
- Quality tests for the overview, focused dataset case management, experiment controls, gate-first result view, mode metrics, and failure diagnosis.
- Auth tests for backend login/signup integration and session validation.
- Settings tests for CI API key creation, one-time secret display, listing, and revoke behavior.
- CI Gates tests for run history, failed-gate reports, metric deltas, and GitHub Actions setup copy.
- Overview, Reports, and Settings page tests should grow as those workflows deepen.
- Playwright tests for upload, focused chunk inspection, cited retrieval evidence, run reruns, responsive workbench layouts, and a real memory-backed login → upload → index → retrieve → debug → compare → Quality workflow.
- Marketing tests for failure-stage tabs, retrieval-mode fixtures, product-tour tabs, mobile navigation, keyboard traversal, reduced motion, CLS, horizontal overflow, and responsive screenshot generation.
- Frontend API client tests for structured JSON, plain-text, and empty error responses.

Feature tests live with implementations under `apps/web/src/features/workbench/<domain>`. Files under `apps/web/src/pages` are thin route wrappers and are not the primary home for workflow tests. Pure feature utilities should be tested without rendering React.

Browser smoke test:

```sh
cd apps/web
npx playwright test
```

Generate the three landing review captures at 1440x1100, 1024x900, and 390x900:

```sh
cd apps/web
npm run screenshots:landing
```

## Documentation Check

When changing commands, paths, or architecture, update:

- `README.md`
- `docs/development.md`
- `docs/eval-lab.md`
- `docs/auth-and-workspaces.md`
- `docs/ci-eval-workflows.md`
- `docs/trace-debugger.md`
- `docs/rag-audit-reports.md`
- `docs/rag-invariants.md`
- `docs/privacy-review-checklist.md`
- `docs/logging-redaction.md`
- `docs/technical-handbook.md`
- `docs/frontend-architecture.md`
- Relevant ADRs in `docs/adr`

Generate and visually check the handbook PDF when architecture or API documentation changes:

```sh
just docs-pdf
```
