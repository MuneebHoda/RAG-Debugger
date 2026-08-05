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
- Local retrieval tests for token normalization, lexical scoring, vector scoring, hybrid missing-embedding behavior, phrase boosts, section/path boosts, deduplication, quality flags, evidence strength, insufficient evidence, cited evidence summaries, weak/duplicate evidence diagnosis, score margins, hybrid disagreement, citations, and expected evidence.
- Retrieval eval tests for recall@k, precision@k, MRR, citation coverage, top-hit rank, weak evidence counts, missing embedding failures, deterministic failure labels, and pass/fail calculation.
- Public regression fixtures for support knowledge bases, policy documents, and technical documentation, with expected retrieval, trace, and Eval Lab outcomes under `fixtures/`.
- API retrieval tests for all-doc search, document filtering, top-k, no-match response, embedding status/indexing, missing embeddings, lexical fallback mode, eval creation, eval run persistence, and request validation.
- Trace tests for trace construction, legacy failure-label compatibility, legacy snapshot enrichment, rerun diagnosis comparison, trace creation from retrieval runs, workspace-scoped listing/detail/reruns, report handoff, and equivalent foreign/missing errors.
- Eval Lab API tests for dataset CRUD, case create/update/delete, legacy case backfill, cross-mode experiments, experiment comparison, regression history, trend summaries, gate evaluation, failure diagnosis, and two-workspace evidence/eval isolation.
- Auth and workspace tests for signup, login, logout, current-user, session cookies, duplicate email behavior, membership role, protected workbench routes, and production-parity session middleware in API tests.
- API key and CI eval tests for one-time secret generation, hashed storage, scoped authorization, revoke behavior, CI run persistence, gate failure status, and `fail_on_gate`.
- API error contract tests for structured 400, 401, 404, and sanitized internal/storage responses.
- Report API tests for session auth, trace/experiment/CI creation, default privacy, workspace-scoped list/detail, Markdown headers, missing reports, and full-local export rejection.
- MemoryStore contract coverage for health, project bootstrap, source/document/chunk persistence, chunk ordering, embedding candidates, and embedding status transitions.
- Evidence repository contract coverage for direct ordered ID resolution, bounded deterministic search, exclusions, source/path/section/body/ID matching, compact UTF-8 previews, and empty-query browsing across MemoryStore and migrated Postgres. MemoryStore instrumentation verifies examined browse entries over 10,000 documents and 60,000 chunks, ordered-index synchronization after replacement/removal/demo merging, direct exact-ID behavior, and bounded text-search retention. An API call-count fake guards the fixed lookup shape: one document resolution, one chunk resolution, and at most one candidate search; case validation never invokes candidate search.
- Workspace isolation contracts run against MemoryStore and migrated Postgres for corpus evidence, embedding status/writes, retrieval candidates/runs, traces, datasets, cases, experiments, and legacy eval runs. Two-workspace API tests cover trace creation/list/detail/rerun/report paths, Overview counts, and embedding mutation. A temporary-database migration test verifies singleton ownership backfill and ambiguous multi-workspace quarantine for projects, Eval records, retrieval runs, and traces.
- ReportRepository contract coverage for snapshot ordering, duplicate IDs, missing reports, and workspace isolation.
- Domain serialization tests as contracts become public.
- Audit-report contract tests for source discriminators, privacy-mode wire values, optional evidence metadata, RFC3339 timestamps, and JSON round trips.
- Audit-report builder tests for deterministic trace/eval/CI output, metadata redaction, bounded snippets, rerun and regression context, recommendation deduplication, and invalid trace sources.
- Audit-report Markdown snapshot tests for exact trace/eval/CI structure, metadata-only redaction, snippets-allowed escaping and bounds, deterministic ordering, and full-local export rejection.

DB-backed integration checks require local Postgres:

```sh
docker compose up -d postgres
sqlx migrate run
```

Run the focused in-memory storage contract with:

```sh
cargo test -p rag-debugger-storage --test memory_store_contract
cargo test -p rag-debugger-storage --test report_store_contract
cargo test -p rag-debugger-storage --test evidence_repository_contract
cargo test -p rag-debugger-storage --test runtime_workspace_contract
DATABASE_URL=postgres://postgres:postgres@localhost:5432/rag_debugger cargo test -p rag-debugger-storage --test evidence_repository_contract postgres_evidence_repository_is_deterministic_and_bounded -- --ignored
DATABASE_URL=postgres://postgres:postgres@localhost:5432/rag_debugger cargo test -p rag-debugger-storage --test eval_workspace_contract postgres_eval_repository_enforces_workspace_ownership -- --ignored
DATABASE_URL=postgres://postgres:postgres@localhost:5432/rag_debugger cargo test -p rag-debugger-storage --test runtime_workspace_contract postgres_runtime_repository_enforces_workspace_ownership -- --ignored
DATABASE_URL=postgres://postgres:postgres@localhost:5432/rag_debugger cargo test -p rag-debugger-storage --test workspace_migration workspace_ownership_migration_backfills_singletons_and_quarantines_ambiguity -- --ignored
DATABASE_URL=postgres://postgres:postgres@localhost:5432/rag_debugger cargo test -p rag-debugger-storage postgres_evidence_query_plans_are_index_compatible -- --ignored
cargo test -p rag-debugger-rag --test report_markdown_snapshots
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

## Autonomous Engineering

Run the deterministic automation suite without an OpenAI key or GitHub write token:

```sh
cd apps/web
npm run autonomy:check
```

Fixtures cover model/effort pinning, structured-output schemas, duplicate proposals, sanitized planner inventory, issue ordering, deterministic branch names, protected/sensitive/artifact paths, size approvals, exact changed-file declarations, pre-artifact secret rejection, path traversal, control-character paths, and symlinks. Static workflow checks enforce immutable action pins, read-only default permissions, diagnostic-only dispatch, disabled schedule gates, one model call, lifecycle pause checks, generation/publication credential separation, workspace-write plus `drop-sudo`, and the absence of automatic merge/ready/close operations.

Tests never call Codex, consume paid tokens, mutate repository settings, apply labels, create branches, or open issues/PRs. The Issue #27 fixture creates a temporary Git repository and exercises the real reviewed event, sanitized context, trusted bootstrap capability, claimability, candidate capture/application, exact base SHA, sensitive-path policy, attestation, structural checks, diff quality check, and deterministic draft-PR payload. Negative fixtures cover ordinary sensitive changes, out-of-capability Issue #27 changes, duplicate bootstrap events, failed-versus-published state, one-call/no-retry policy, traversal, symlink and file replacement, in-place mutation, hash mismatch, malicious outbound destinations, credentials, encoded paths, and redirect refusal. Live validation remains the reviewed Issue #27 bootstrap after App, secrets, variables, and budget controls are configured.

Expected coverage in the scaffold:

- App shell and workbench navigation render tests.
- Corpus render and focused document-detail tests, including the advanced structured chunking control.
- Test Retrieval render and mocked query tests, including one mode control, collapsed advanced settings, evidence summary, score bars, citations, and direct debugger navigation.
- Runs tests for search/list navigation, primary diagnosis, backend failure labels and recommendations, per-evidence score explanations, rerun diagnosis comparison, and explicit dataset/evidence selection for Quality.
- Quality tests for the overview, focused dataset case management, experiment controls, gate-first result view, mode metrics, and failure diagnosis.
- Eval Lab expected-evidence tests for explicit Search/Enter submission, local short-query rejection, cancellable stale requests, independent candidate limits, bounded submitted-ID work, exact-UUID routing, searchable document/chunk lookup, compact previews, deduplication, metadata-error preservation, stale evidence labels and removal, omitted-versus-present PATCH semantics, nullable note clearing, atomic repair failures, cancellation, immediate reopen during delayed refetch, source-keyed Retrieval/Trace drafts, immutable POST payloads, transition blocking, duplicate-submit guards, dataset readiness, live-region feedback, keyboard toggles/removal/clear/save, CSS-module coverage, and text-labeled evidence states.
- Auth tests for backend login/signup integration and session validation.
- Settings tests for CI API key creation, one-time secret display, listing, and revoke behavior.
- CI Gates tests for run history, failed-gate reports, metric deltas, and GitHub Actions setup copy.
- Overview and Settings page tests should grow as those workflows deepen.
- Audit Reports feature tests cover generated-list and candidate rendering, source-driven creation, detail rendering, privacy states, structured errors, and clipboard export behavior.
- Audit-report integration tests cover trace, experiment, and CI source payloads, explicit privacy selection, direct detail navigation, duplicate-submit blocking, and inline mutation errors.
- Playwright tests for upload, focused chunk inspection, cited retrieval evidence, run reruns, responsive workbench layouts, and a real memory-backed login → upload → index → retrieve → debug → compare → keyboard-select exact-chunk and document evidence → save → verify Quality workflow.
- Marketing tests for failure-stage tabs, retrieval-mode fixtures, product-tour tabs, mobile navigation, keyboard traversal, reduced motion, CLS, horizontal overflow, and responsive screenshot generation.
- Frontend API client tests for structured JSON, plain-text, and empty error responses.
- Guided-demo contract and service tests for stable query IDs, compiled fixture preparation, deterministic IDs, partial-load repair, repeat-load idempotency, and workspace isolation.
- Guided-demo API tests for authentication, `201` first load, `200` repeat load, exactly three documents, and stable chunk counts.
- Guided-demo UI tests for each checklist transition, source-scoped indexing, query-ID resolution, demo-source preselection, and Markdown copy/download privacy behavior.
- The real Playwright demo flow covers login → load sample → index → suggested query → debug run → metadata-only report → copy/download Markdown.
- Workbench shell tests treat canonical navigation order, active parent routes, breadcrumbs, actionable empty states, mobile focus recovery, and the dedicated CI Runs query view as compatibility contracts.
- Workbench workflow tests verify that Home, Corpus, Retrieval, Trace Debugger, Eval Lab, CI Runs, Audit Reports, and Settings explain their page purpose, quality-loop position, and recommended next action.
- Eval Lab UI tests verify trend cards, dataset experiment history, experiment regression panels, explicit baseline selection, compatibility warnings, no-baseline states, failed-case diagnosis, and audit-report actions without duplicating backend regression logic.
- Expected-evidence tests live under `features/workbench/eval-lab/evidence` and cover picker behavior, explicit expectation-only versus completed-retrieval contexts, real parent-document resolution, zero-hit results, wrong-chunk classification, unavailable metadata, and contradiction prevention separately from page rendering.

The migrated Postgres gate runs the evidence parity contract, Eval and runtime workspace-isolation contracts, legacy ownership migration contract, and `postgres_evidence_query_plans_are_index_compatible`. The plan fixture contains 20,000 documents and 60,000 chunks and verifies browse B-tree indexes, exact primary-key paths, selective path/section/body trigram indexes, and the absence of a full-corpus chunk scan or sort for limit-one browsing. The test does not disable sequential scans or other planner choices. MemoryStore has a separate measured-work fixture: limit-one empty browse examines one ordered entry, excluded prefixes are counted precisely, exact UUIDs bypass candidate scans, and linear text search retains only its requested top-k.

Feature tests live with implementations under `apps/web/src/features/workbench/<domain>`. Files under `apps/web/src/pages` are thin route wrappers and are not the primary home for workflow tests. Pure feature utilities should be tested without rendering React.

Browser smoke test:

```sh
cd apps/web
npx playwright test
```

Run the focused workbench route, accessibility, responsive-layout, and hostile
content gates with:

```sh
cd apps/web
npx playwright test tests/e2e/workbench-quality.spec.ts
```

The suite covers Home, Corpus, Retrieval, Trace Debugger, Eval Lab, CI Runs,
Audit Reports, and Settings at 1440, 1280, 1024, 768, and 390 pixel widths.
Typed fixtures verify long paths, IDs, queries, snippets, score breakdowns,
labels, report titles, and API key names without external services or customer
data. Child-to-panel containment checks catch internal collapse even when the
document itself has no horizontal overflow. See
[`docs/workbench-ui-quality.md`](workbench-ui-quality.md) for the merge
checklist and assertion rules.

Generate ignored workbench review captures with:

```sh
cd apps/web
npm run screenshots:workbench
```

Generate the five landing review captures at 1440x1100, 1280x900, 1024x900,
768x900, and 390x900:

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
- `docs/guided-demo.md`
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
