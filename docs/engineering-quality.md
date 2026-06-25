# Engineering Quality

CorpusLab uses GitHub as the engineering source of truth. Issues define work, pull requests change code, CI enforces quality, milestones group releases, and changelog entries preserve product history.

## Workflow

1. Create a GitHub issue for every feature, bug, refactor, security concern, UI polish pass, or performance task.
2. Create a branch from `main` using a short conventional prefix:
   - `feat/<short-name>`
   - `fix/<short-name>`
   - `refactor/<short-name>`
   - `docs/<short-name>`
   - `test/<short-name>`
3. Open a pull request into `main`.
4. Fill out the PR template, including tests run, migration notes, docs, and rollback plan.
5. Merge only after CI passes.
6. Squash merge with a conventional title.

## Versioning

CorpusLab uses semantic pre-release versioning before launch:

- `v0.1.0`: baseline product checkpoint.
- `v0.2.0`: next coherent product milestone.
- `v0.2.1`: patch-level fix inside a milestone.
- `v0.3.0-rc.1`: release candidate.

Every milestone release should update `CHANGELOG.md` and create a GitHub Release.

## GitHub Project Board

Use one project board with these columns:

- `Backlog`
- `Ready`
- `In Progress`
- `Review`
- `Done`

Recommended milestones:

- `v0.1 Quality Baseline`
- `v0.2 Eval Lab Hardening`
- `v0.3 Hosted Foundations`
- `v0.4 GPU/HPC Workers`

Recommended labels:

- Areas: `area/api`, `area/web`, `area/rag`, `area/storage`, `area/docs`
- Types: `type/bug`, `type/feature`, `type/refactor`, `type/test`, `type/security`, `type/performance`
- Priorities: `priority/p0`, `priority/p1`, `priority/p2`, `priority/p3`

## Local Quality Gates

Use the fast gate while developing:

```sh
just check
```

Use the release gate before a baseline PR or milestone release:

```sh
just full-check
```

`just full-check` runs Rust checks, web checks, Playwright, handbook PDF generation, and SQLx migrations against local Postgres.

## Code Quality Rules

- Keep public API changes backward-compatible within `/api/v1` unless a changelog entry and migration note explain the break.
- Add or update tests at the lowest useful layer.
- Keep raw documents local by default and document privacy changes.
- Add an ADR for architecture, storage, security, API, or deployment decisions.
- Do not add a large file without either splitting it in the same PR or creating a linked refactor issue.
- Prefer small domain modules over broad files such as one giant API client, storage adapter, or route component.

## Current Cleanup Targets

The product is moving fast, so these hot spots should be split over dedicated refactor PRs:

- `apps/web/src/lib/api/client.ts`: move implementation behind the new domain API modules instead of adding more exports to the internal client.
- `apps/web/src/features/workbench/workbench.css`: move route-specific rules into CSS modules.
- `apps/web/src/pages/RetrievalPage.tsx`, `EvalsPage.tsx`, `TracesPage.tsx`, and `SourcesPage.tsx`: move panels/forms/cards into feature folders.
- `crates/storage/src/postgres.rs`: split persistence by bounded context.
- `crates/storage/src/repository.rs`: split repository traits by domain as the API grows.

These should be refactors with no product behavior changes.
