# Changelog

All notable CorpusLab changes should be recorded here.

This project uses semantic pre-release versioning while the product is pre-launch:

- `0.x.y` for product milestones.
- `0.x.y-rc.n` for release candidates.
- Git tags use `v` prefixes, for example `v0.1.0`.

## [Unreleased]

### Security

- Upgraded PDF extraction to remove a known `lopdf` vulnerability, marked internal Rust packages as non-publishable, and refined Cargo Deny to block direct unmaintained dependencies while keeping transitive unmaintained, duplicate-version, and yanked-crate findings visible as nonblocking warnings.
- Added a strict Cargo Deny gate for RustSec advisories, dependency bans, approved licenses, and trusted dependency sources.
- Removed compiled bootstrap and Postgres passwords from API runtime defaults, require explicit non-empty bootstrap credentials and Postgres configuration, and stopped displaying or prefilling login credentials in the web app.

### Added

- Versioned golden dataset schema v1 with stable persisted case keys, deterministic privacy-aware JSON export, mandatory dry-run import modes, atomic workspace-scoped MemoryStore/Postgres application, CI-key validation/import, and practical Eval Lab transfer controls.
- Versioned immutable Eval Lab experiment provenance with deterministic canonical SHA-256 identities for datasets, corpus/document checksums, chunking/chunk sets, embedding configuration/indexes, retrieval/scoring/filters/runtime flags, and privacy-safe build/CI metadata.
- Typed baseline compatibility with machine-readable reasons and changed fields, strict fully compatible automatic selection, explicit warned cross-configuration comparisons, legacy-unknown handling, Experiment Detail provenance, and privacy-safe audit-report context.
- Local-first launch-readiness package with a reproducible design-partner demo, onboarding guide, technical one-pager, sanitized feedback template, honest limitations, and discoverable README/handbook navigation.
- Workspace-scoped native and OTLP/HTTP protobuf trace ingestion with idempotent span merging, deterministic monotonic status, pre-persistence privacy enforcement, privacy-safe span names and kinds, imported Trace Debugger views, privacy-provenance-preserving full-local Eval conversion, permitted audit reports, and local SDK and Collector examples.
- Hardened imported-trace retries with unique aggregate evidence ranks, preserved rerun history and timestamps, server-side rerun rejection, strict CorpusLab scores with partial third-party score mapping, collision-aware errors, complete ingestion metadata constraints, and isolated log-redaction coverage.
- Complete CI release-gate workflow with explicit hashed-key onboarding, privacy-safe GitHub Actions summaries, persisted Eval Lab v2 regression context, focused failed-run diagnosis, and metadata-only audit-report creation.
- Repository governance baseline with rendered-Markdown exact-link issue-policy validation, private security-reporting and containment guidance, actionable bug and feature forms, explicit CODEOWNERS coverage, and clean-worktree exact-commit release verification.
- Advisory CodeRabbit reviews for ready pull requests with repository-owned settings, high-level summaries, nonblocking status, disabled code-generation finishing touches, and a documented external engineering-data boundary.
- Informational Codecov project and changed-line coverage reporting for the complete Rust workspace and all production frontend TypeScript, with separate OIDC-authenticated Rust and web uploads and no merge-blocking percentage threshold.
- Workspace-isolation contracts for MemoryStore, migrated Postgres, authenticated two-workspace APIs, runtime retrieval/trace/embedding paths, and legacy ownership backfill/quarantine behavior.
- Evidence-search browse indexes and migrated query-plan regression coverage over a 20,000-document/60,000-chunk fixture.
- Deterministic MemoryStore/Postgres evidence repositories with capped direct-ID resolution, bounded browse/results, compact UTF-8 previews, and trigram-indexed Postgres corpus search.
- Expected-evidence editor for Eval Lab with authenticated document/chunk lookup, readable multi-evidence selection, independent exact-chunk versus document-level expectations, stale evidence warnings, shared Retrieval/Trace save-to-Quality panels, and text-labeled evidence state visualization.
- Eval Lab v2 regression history with dataset experiment timelines, trend summaries, baseline comparisons, newly failed/recovered case detection, and regression-aware audit reports.
- Shared workbench workflow guidance that explains each page's purpose, position in the RAG quality loop, recommended next action, and retrieval-quality impact.
- Shared authenticated-workbench visual primitives for panels, toolbars, status pills, and dense metric cards, with focused component tests.
- Dependency-free workbench UI quality gates covering semantic route contracts, desktop/tablet/mobile overflow, keyboard navigation, reduced motion, hostile technical strings, and opt-in review screenshots.
- A scenario-reactive SVG Evidence Reactor that projects document, chunk, ranking, answerability, CI-gate, and audit-report state around the landing command center with responsive and reduced-motion behavior.
- An animated, keyboard-accessible RAG diagnosis command center on the public landing page with ranked evidence, answerability states, failure labels, score lineage, CI gate signals, audit-report readiness, explicit playback control, and reduced-motion behavior.
- A deterministic answerability gate that keeps broad retrieval candidates visible while allowing Evidence Summary citations only from directly supporting chunk body text.
- Supported answers now remain primary in Retrieval and Trace Detail while weak lower-ranked candidates stay visible as retrieval-quality warnings.
- Deterministic Debugger Intelligence v2 with strong/mixed/weak/failing outcomes, typed failure labels, per-evidence score explanations, actionable remediation, rerun diagnosis deltas, Eval Lab expected-evidence context, and privacy-safe audit-report reuse.
- Authenticated, idempotent guided demo loading with three versioned public fixtures, deterministic workspace-owned IDs, source-specific progress, suggested diagnostic queries, and a six-step Home workflow through Markdown audit export.
- Professional deterministic Markdown audit exports with escaped user content, privacy enforcement, ordered sections, and checked-in trace/eval/CI snapshot fixtures.
- Privacy-aware audit-report actions on Trace Detail, Eval experiment detail, and failed CI gates with direct report navigation and duplicate-submit protection.
- Audit Reports workbench list, source-driven creation, focused report detail, privacy classification, and Markdown copy workflow.
- Authenticated workspace-scoped audit report creation, list/detail, and privacy-enforced Markdown export APIs.
- Workspace-scoped MemoryStore and Postgres persistence for append-only RAG audit report snapshots.
- Deterministic privacy-aware RAG audit report builders for traces, Eval Lab experiments, and CI regression runs.
- Additive RAG Audit Report domain contracts for trace, Eval Lab, CI, and manual report sources with explicit privacy modes and deterministic evidence links.
- Privacy-review triggers and a logging/redaction allowlist covering corpus data, queries, traces, credentials, exports, and hosted sync.
- Documented retrieval, trace, Eval Lab, and privacy invariants with synthetic public regression corpora and deterministic failure-label coverage.
- Documentation maintenance, dependency, and generated-file policies plus aligned Rust, web, and CI `just` gates.
- Repository-level agent engineering rules and a frontend architecture guide covering route, feature, API, CSS, and testing boundaries.
- Engineering quality baseline for GitHub-first execution, CI gates, PR templates, issue templates, and release discipline.
- Guided workbench navigation, recoverable route errors, live setup progress, and shared query-state foundations.
- Focused document, run, Quality dataset, and Quality experiment routes.
- A memory-backed Playwright workflow covering login through Quality-case creation.
- Premium interactive landing sections for failure diagnosis, retrieval-mode comparison, capability storytelling, product tours, enterprise trust, and responsive navigation.
- Route-isolated Motion animations, reduced-motion behavior, responsive landing screenshots, and JavaScript/CSS gzip budgets.

### Changed

- Manual and CI Eval Lab experiment execution now rejects datasets above a shared 250-case limit before snapshotting, provenance, retrieval, result allocation, or persistence, and uses fallible bounded result reservation.
- CI automatic baseline discovery now uses a workspace- and dataset/config-scoped repository lookup across all earlier runs, selecting only exact schema-and-identity provenance matches with deterministic MemoryStore/Postgres ordering.
- Manual and CI Eval Lab runs now share one frozen workspace-scoped source/chunk snapshot, preserve additive provenance in existing experiment JSON, and enforce append-only experiment IDs plus MemoryStore/Postgres provenance ownership parity without a database migration.
- Corpus evidence, embedding status and writes, retrieval candidates and saved runs, traces and reruns, Eval Lab datasets/cases/experiments/runs, CI reads, overview metrics, demo progress, and report inputs now enforce authenticated workspace ownership inside storage; cross-workspace identifiers use the same unresolved, unavailable, or not-found behavior as nonexistent resources.
- Retrieval and Trace save-to-Quality panels now key drafts and mutations to the active run or trace, preserve dataset intent across source changes, block stale submissions during retrieval transitions, expose keyboard-operable evidence toggles and announced async states, and verify submitted payloads against the current source.
- Expected-evidence lookup now caps submitted ID work before storage access, routes browse/exact/text searches explicitly, uses synchronized ordered MemoryStore browse indexes and bounded text-search retention, follows index-compatible Postgres paths, rejects unsafe short text without clearing picker results, and preserves removable selections when metadata is unavailable.
- Eval Lab evidence search now runs only on explicit Search or Enter submission, keeps selected evidence outside candidate limits, and avoids corpus-wide document-by-document chunk loading.
- Eval Lab case notes now distinguish omitted, replacement, and explicit-null PATCH values, while successful edits update the dataset cache synchronously so immediate reopen uses the saved case before background refetch completes.
- Eval Lab case editing now preserves unchanged legacy stale evidence during scalar-only updates, exposes explicit stale-item removal, validates modified selections atomically, supports intentional evidence clearing, and restores persisted drafts on cancellation.
- Eval Lab evidence states now distinguish saved expectations from completed retrievals, resolve experiment hits through real chunk/document metadata, and represent unavailable metadata without false missing, retrieved, or wrong-chunk claims.
- Eval Lab experiment detail now supports URL-persisted baseline selection, compatibility warnings, full regression-category rendering, and a clear no-baseline state.
- Home, Corpus, Retrieval, Trace Debugger, Eval Lab, CI Runs, Audit Reports, and Settings now make the Upload → Chunk → Embed → Retrieve → Trace → Eval → CI gate → Report loop explicit for first-time users.
- Core workbench pages now use the shared visual-system primitives for panels, metrics, toolbars, and status labels, reducing one-off CSS and route-level styling drift.
- Workbench panels now preserve readable empty states, visible mobile tabs, stable dense grids, and contained technical content across five responsive viewports, with populated detail screenshots and component-level geometry gates.
- The authenticated workbench now follows one typed Setup → Debug → Quality → Share → Admin information architecture with canonical route labels, linked breadcrumbs, a focused CI Runs view, consistent page headers, actionable empty states, mobile focus management, and route-scoped legacy styles.
- Playwright coverage is split by marketing, mocked workbench, real workflow, and quality-gate ownership, with shared typed fixtures and exact endpoint mocks.
- The post-hero landing experience now follows one CorpusLab-specific story from unsupported retrieval through deterministic diagnosis, Eval Lab regression, CI gating, metadata-only audit reporting, and local-first privacy controls.
- The animated landing command center now uses a more spacious responsive stage, while the landing-only header adapts between hero, dark, and light page surfaces as users scroll.
- Retrieval workbench orchestration now uses a domain hook, focused control panels, and tested pure filter utilities without changing behavior.
- Runs and Trace Debugger UI now use focused components, a URL-backed trace hook, and tested filtering, recommendation, and route-loading behavior.
- Storage persistence now exposes bounded repository traits with a method-free ingestion compatibility composite and MemoryStore contract coverage.
- API route composition now has a dedicated module, and structured errors are parsed by the web client while internal storage details remain private.
- CI now covers frontend formatting, Playwright smoke tests, technical handbook PDF generation, and SQLx migration checks.
- API timestamps now use RFC3339 strings while remaining compatible with legacy persisted timestamp arrays.
- Corpus and Test retrieval now lead with one primary task and disclose chunking, indexing, filters, and ranking controls under Advanced sections.
- Runs now separate search from debugging; Quality now separates overview, dataset management, and experiment results.
- Reports prioritize actionable diagnoses, Settings use task-focused tabs, and route-specific styling is isolated in CSS modules.
- The public landing page now uses an editorial full-width composition instead of repeated feature-card grids.
- Quality experiment controls now use explicit layout regions and safe wrapping at desktop, tablet, and mobile widths.

## [0.1.0] - Baseline Product Checkpoint

### Added

- CorpusLab workbench with ingestion, chunk preview, local embeddings, retrieval playground, trace debugger, Eval Lab, reports, settings, and marketing pages.
- Rust workspace with `apps/api`, `crates/core`, `crates/rag`, and `crates/storage`.
- React + Vite + TypeScript web app with strict typing, Vitest, Playwright, and CSS modules for newer surfaces.
- Postgres migrations for sources, documents, chunks, retrieval runs, embeddings, traces, evals, and Eval Lab.
- Engineering handbook and PDF generation workflow.
