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
- API retrieval tests for all-doc search, document filtering, top-k, no-match response, embedding status/indexing, missing embeddings, lexical fallback mode, eval creation, eval run persistence, and request validation.
- Trace tests for trace construction, failure label assignment, rerun comparison, trace creation from retrieval runs, trace listing/detail, rerun API behavior, and missing-trace errors.
- Eval Lab API tests for dataset CRUD, case create/update/delete, legacy case backfill, cross-mode experiments, experiment comparison, gate evaluation, and failure diagnosis.
- Auth and workspace tests for signup, login, logout, current-user, session cookies, duplicate email behavior, membership role, and protected workbench routes.
- API key and CI eval tests for one-time secret generation, hashed storage, scoped authorization, revoke behavior, CI run persistence, gate failure status, and `fail_on_gate`.
- Domain serialization tests as contracts become public.

DB-backed integration checks require local Postgres:

```sh
docker compose up -d postgres
sqlx migrate run
```

## Web

Run:

```sh
cd apps/web
npm run typecheck
npm run lint
npm test
npm run build
```

Expected coverage in the scaffold:

- App shell and workbench navigation render tests.
- Sources page render test, including the structured chunking control.
- Retrieval page render and mocked query tests, including mode controls, embedding status, evidence summary, score bars, citations, trace saving, and save-to-Eval-Lab.
- Trace Debugger tests for navigation, trace list/detail, timeline spans, failure labels, rerun controls, comparison metrics, explainer cards, and save-to-Eval-Lab.
- Eval Lab tests for dataset lists, case editing surface, run controls, experiment mode matrix, gate status, and failure diagnosis.
- Auth tests for backend login/signup integration and session validation.
- Settings tests for CI API key creation, one-time secret display, listing, and revoke behavior.
- CI Gates tests for run history, failed-gate reports, metric deltas, and GitHub Actions setup copy.
- Overview, Reports, and Settings page tests should grow as those workflows deepen.
- Playwright smoke tests for upload, chunk metadata preview, cited retrieval evidence, trace reruns, Eval Lab experiment, report view, and settings/config display.

Browser smoke test:

```sh
cd apps/web
npx playwright test
```

## Documentation Check

When changing commands, paths, or architecture, update:

- `README.md`
- `docs/development.md`
- `docs/eval-lab.md`
- `docs/auth-and-workspaces.md`
- `docs/ci-eval-workflows.md`
- `docs/trace-debugger.md`
- `docs/technical-handbook.md`
- Relevant ADRs in `docs/adr`

Generate and visually check the handbook PDF when architecture or API documentation changes:

```sh
just docs-pdf
```
