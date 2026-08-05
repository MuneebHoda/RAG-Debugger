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

Use `just rust-check` or `just web-check` for focused iteration. `just check` composes both fast gates. `just ci-check` is the explicit release-equivalent gate, and `just full-check` remains its backward-compatible alias.

## Coverage Baseline

The separate `Coverage` workflow measures the complete locked Rust workspace with all features and every production TypeScript and TSX file under `apps/web/src`. Rust uses `cargo-llvm-cov 0.8.7`; the web uses Vitest's V8 provider through the matching `@vitest/coverage-v8 4.1.9` development dependency. Both jobs publish explicit LCOV reports to Codecov with `rust` and `web` flags.

Frontend test/spec files, `setupTests.ts`, and TypeScript declaration files are excluded because they are test infrastructure or type-only inputs rather than shipped application behavior. Untested production files are deliberately included. Rust coverage excludes dependency, compiler, generated build, test-harness, example, and benchmark sources through `cargo-llvm-cov` defaults while retaining all workspace production crates.

Codecov project and changed-line coverage statuses are informational while the team collects a representative baseline. No coverage percentage, target, or threshold currently blocks merging, and the coverage workflow is not a required branch-protection check. The jobs still fail when report generation, path validation, or the OIDC-authenticated upload breaks so coverage infrastructure failures remain visible.

Generate the reports locally with:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --version 0.8.7 --locked
cargo llvm-cov --workspace --all-features --locked \
  --lcov --output-path target/llvm-cov/lcov.info
cd apps/web && npm run test:coverage
```

Generated LCOV files remain ignored. Coverage thresholds will be considered only after reviewing the real Rust and web baselines, changed-line behavior, and consistently uncovered product areas across multiple pull requests.

## Automated Pull Request Review

CodeRabbit provides an advisory review of ready pull requests using the repository-owned `.coderabbit.yaml`. The `chill` profile prioritizes correctness, security, and maintainability findings without turning stylistic preferences into merge blockers. Reviews include a high-level summary and visible review status; draft pull requests are skipped until they are marked ready.

CodeRabbit does not use the request-changes workflow and is not a required branch-protection check. Automated code edits, docstrings, unit tests, simplification, CI fixes, conflict resolution, prompts for other AI agents, poems, fortunes, and automatic chat replies are disabled. A contributor can still request a manual review through CodeRabbit's documented PR command when needed.

The GitHub App is limited to this public repository. It processes repository source, pull request diffs, review metadata, and check results as an external engineering provider, but it has no CorpusLab runtime, database, workspace, or customer-data integration. GitHub secret scanning and push protection are enabled before external review. The protected `main` ruleset still requires pull requests and the existing CI checks; CodeRabbit is advisory, so feature-branch changes to its configuration cannot weaken those gates.

CodeRabbit's default cache and knowledge-base retention remain explicitly enabled. Review caches expire within seven days, while repository learnings and pull request context remain until an administrator deletes them or opts out. The repository owner reviews the provider's maintained subprocessor register and retention controls. The full provider boundary, deletion procedure, and rollback decision are recorded in [ADR 0005](adr/0005-coderabbit-review-provider.md).

## Pull Request Dependency Gate

Every pull request runs the `Dependency Review` check. It compares dependency changes with the GitHub Advisory Database and fails when the pull request introduces a known High or Critical vulnerability in runtime, development, or unknown dependency scopes.

The workflow uses read-only repository permissions, disables persisted checkout credentials, pins actions to immutable commit SHAs, and cancels superseded runs. Review the reported dependency diff before merging even when the check passes; severity is a screening threshold, not a complete risk assessment.

## Rust Dependency Policy Gate

The `Cargo Deny` check evaluates the complete locked Rust workspace on every pull request, every push to `main`, and a weekly schedule. It checks RustSec advisories, dependency bans, license policy, and dependency sources. Known vulnerabilities block regardless of whether they are direct or transitive dependencies.

Unmaintained advisories block when they apply to a direct dependency of a workspace package. Transitive unmaintained dependencies remain maintenance concerns but do not currently block CI. Versionless path dependencies are permitted only inside workspace packages explicitly marked as private with `publish = false`; wildcard registry dependencies remain denied.

Unapproved licenses, unknown registries, and unknown Git sources block CI. Duplicate transitive crate versions and yanked crates, including the current `spin 0.9.8` dependency, remain visible as warnings so they can be reduced incrementally without weakening the security gate.

Do not ignore an advisory or add a license exception simply to make CI pass. Any necessary license exception must name an exact crate and version, explain why the license is required and compatible with the project, and receive explicit review. Advisory exceptions require the same written justification and security review, including the exposure, available mitigations, and a removal plan.

Run the same policy locally with Cargo Deny 0.20.2:

```sh
cargo deny --workspace --all-features --locked check advisories bans licenses sources
```

## Code Quality Rules

- AI-agent and agent-assisted changes must follow the root `AGENTS.md` rules.
- Keep public API changes backward-compatible within `/api/v1` unless a changelog entry and migration note explain the break.
- Add or update tests at the lowest useful layer.
- Keep raw documents local by default and document privacy changes.
- Add an ADR for architecture, storage, security, API, or deployment decisions.
- Do not add a large file without either splitting it in the same PR or creating a linked refactor issue.
- Prefer small domain modules over broad files such as one giant API client, storage adapter, or route component.
- Follow `docs/frontend-architecture.md` for web feature, API, styling, and testing boundaries.
- Complete `docs/privacy-review-checklist.md` for changes involving data movement, external services, telemetry, authentication, retention, sharing, or exports.
- Follow `docs/logging-redaction.md` for every log, telemetry event, support bundle, and CI diagnostic.

## Privacy Review Gate

Privacy review is part of engineering review, not a post-release compliance task. A pull request that changes a privacy boundary must name the affected data classes, local and external destinations, access control, retention/deletion behavior, redaction, user control, and rollback path. Hosted sync, an external model/provider, auth-provider replacement, or a change to local-first defaults requires an ADR.

The current logging audit permits structured startup metadata and prohibits raw corpus, query, prompt, answer, vector, credential, header, and cookie data. Use opaque IDs and aggregate operational fields to correlate failures.

## Dependency Policy

New dependencies require a PR explanation covering need, existing alternatives, runtime or bundle cost, security and maintenance impact, local-first privacy impact, and alternatives considered. Dev-only use of an existing workspace dependency must still be identified.

## Generated Files

Do not commit local databases, uploaded documents, logs, dependency directories, compiler output, coverage, Playwright output, or generated screenshots. Generated documents are excluded unless intentionally versioned; `docs/technical-handbook.pdf` and curated `apps/web/public/product` assets are explicit exceptions.

Follow `docs/doc-maintenance.md` for documentation ownership, ADR triggers, and changelog rules.

## Current Cleanup Targets

The product is moving fast, so these hot spots should be split over dedicated refactor PRs:

- `apps/web/src/features/workbench/workbench.css`: move route-specific rules into CSS modules.
- `apps/web/src/features/workbench/eval-lab/DatasetDetailPage.tsx`: separate case editing from experiment controls and mutations.
- `apps/web/src/features/workbench/sources`: keep corpus upload, library, and document inspection in focused components.

Domain files under `apps/web/src/pages` are route wrappers or compatibility re-exports and should remain thin. The remaining legacy page implementations should move into `apps/web/src/features/workbench/<domain>` through focused refactors. Cleanup targets should not change product behavior unless a separately tested bug is found.

The Retrieval route now follows the target convention: `RetrievalPage.tsx` composes a domain hook, focused control panels, result panels, and tested pure filter utilities.

The Runs and Trace Debugger routes now use a trace query/tab hook, a focused run list, separate summary, failure, evidence, metrics, timeline, rerun, and Quality components, plus tested filter and recommendation utilities.

Storage now exposes bounded health, project, source, document, embedding, retrieval, trace, eval, auth, and CI eval traits. `IngestionRepository` is a method-free compatibility composite limited to the upload workflow, and `AppRepository` composes all application capabilities.

The low-level frontend API client remains transport-only and now parses the backend error envelope into status, code, user-facing message, and raw diagnostic body. API route registration is isolated from handler-module declarations.
