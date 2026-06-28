# Changelog

All notable CorpusLab changes should be recorded here.

This project uses semantic pre-release versioning while the product is pre-launch:

- `0.x.y` for product milestones.
- `0.x.y-rc.n` for release candidates.
- Git tags use `v` prefixes, for example `v0.1.0`.

## [Unreleased]

### Added

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

- Retrieval workbench orchestration now uses a domain hook, focused control panels, and tested pure filter utilities without changing behavior.
- Runs and Trace Debugger UI now use focused components, a URL-backed trace hook, and tested filtering, recommendation, and route-loading behavior.
- Storage persistence now exposes bounded repository traits with a method-free ingestion compatibility composite and MemoryStore contract coverage.
- API route composition now has a dedicated module, and structured errors are parsed by the web client while internal storage details remain private.
- CI now covers frontend formatting, Playwright smoke tests, technical handbook PDF generation, and SQLx migration checks.
- API timestamps now use RFC3339 strings while remaining compatible with legacy persisted timestamp arrays.
- The workbench shell now organizes CorpusLab around Build, Improve, Share, and Workspace workflows.
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
