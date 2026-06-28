# CorpusLab Agent Engineering Rules

CorpusLab is a RAG debugging and corpus observability platform. The goal of this repository is not to generate code quickly. It is to make retrieval failures inspectable, testable, and explainable without weakening privacy or maintainability.

These rules apply to AI coding agents and human contributors using agent-generated changes.

## Product Purpose

Product work should help users answer at least one of these questions:

- What documents and chunks were retrieved?
- Why were those chunks retrieved?
- Which score signals influenced ranking?
- Did the final answer use the right context?
- Where did hallucination, weak evidence, duplicate evidence, missing embeddings, or bad chunking appear?
- Did a change improve or regress retrieval quality?

If a proposed product change does not improve one of these outcomes, clarify its purpose before implementing it.

## Repository Structure

- `apps/api`: Axum API service, route handlers, auth, config, telemetry, and app state.
- `apps/web`: React, Vite, and TypeScript public site and workbench.
- `crates/core`: shared domain contracts and API-facing types.
- `crates/rag`: extraction, chunking, document intelligence, embeddings, retrieval, tracing, and eval behavior.
- `crates/storage`: repository traits plus memory and Postgres adapters.
- `docs`: architecture, development, testing, feature docs, ADRs, roadmap, and handbook.
- `migrations`: SQLx migrations.

Read `docs/frontend-architecture.md` before changing the web application.

## Change Flow

For product features:

1. Update shared domain types in `crates/core` if the API contract changes.
2. Add pure behavior in `crates/rag` when possible.
3. Update repository traits and adapters in `crates/storage` only when persistence changes.
4. Add API handler changes under `apps/api/src/http`.
5. Add frontend API changes under `apps/web/src/lib/api`.
6. Add UI changes under `apps/web/src/features` and keep `apps/web/src/pages` thin.
7. Add tests at the lowest useful layer.
8. Update docs and `CHANGELOG.md` when behavior, commands, architecture, API, or storage changes.

## Scope Control

- Do not rewrite unrelated files.
- Do not rename public API fields unless the task explicitly requires it.
- Keep `/api/v1` backward-compatible unless a migration note and changelog entry explain otherwise.
- Do not add dependencies without documenting the need, alternatives, runtime or bundle impact, security impact, and local-first privacy impact.
- Do not commit large files, local data, test output, or generated artifacts unless explicitly required and intentionally versioned.
- Do not hide errors with broad catches, silent fallbacks, or production `unwrap()` calls.
- Prefer focused modules over giant route, page, API client, or storage files.
- Prefer pure functions in `crates/rag` and `crates/core`.
- Keep raw document handling local-first unless a privacy ADR explicitly changes that boundary.

## Rust Rules

- Run `cargo fmt --all --check`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.
- Run `cargo test --workspace`.
- Use typed domain errors.
- Avoid `unwrap()` and `expect()` in production paths unless a documented invariant makes failure impossible.
- Keep SQL queries near the bounded-context module that owns them.
- New persisted fields require migrations, backward-compatible defaults, and storage coverage.

## Web Rules

- Run `cd apps/web && npm run typecheck`.
- Run `cd apps/web && npm run lint`.
- Run `cd apps/web && npm test -- --run`.
- Run `cd apps/web && npm run build`.
- Keep route pages thin and move feature behavior into domain folders.
- Move stateful orchestration into focused hooks when a page owns several server states or mutations.
- Keep reusable API calls in domain modules under `apps/web/src/lib/api`.
- Keep `apps/web/src/lib/api/client.ts` generic; do not add product-specific requests there.
- Avoid global CSS growth. Prefer route-level and component-level CSS modules.

## Testing Rules

Add tests at the lowest useful layer:

- Pure RAG behavior: unit tests in `crates/rag`.
- Shared contracts: serialization tests in `crates/core`.
- Storage behavior: repository tests in `crates/storage`.
- API behavior: handler and integration tests in `apps/api`.
- UI behavior: Vitest and Testing Library tests in `apps/web`.
- End-to-end workbench flows: Playwright.

Every bug fix should include a regression test unless the PR explains why one is not practical.

## Documentation Rules

Update documentation in the same PR when changing routes, commands, environment variables, migrations, architecture, feature behavior, quality gates, privacy behavior, or setup instructions.

Use an ADR for decisions involving architecture boundaries, storage design, privacy or security, API compatibility, worker or deployment strategy, and hosted versus local behavior.

## Dependency Policy

Every new dependency requires a PR note covering:

- Why the dependency is needed.
- Why existing repository or platform code is insufficient.
- Runtime, binary, or frontend bundle impact.
- Security and maintenance impact.
- Local-first privacy impact.
- Alternatives considered.

Dev-only use of an existing workspace dependency should still be identified, but does not require a new architecture decision.

## Generated File Policy

Do not commit local database files, uploaded documents, logs, `node_modules`, `target`, coverage, Playwright output, or generated screenshots. Generated PDFs are excluded unless the repository explicitly versions them. Current intentional exceptions are `docs/technical-handbook.pdf` and curated assets under `apps/web/public/product`.

## Required Task Summary

Every completed agent task must report:

1. Files changed.
2. Behavior changed.
3. Tests added or updated.
4. Commands run.
5. Documentation updated.
6. Risks.
7. Rollback plan.
8. Follow-up issues.
