# Changelog

All notable CorpusLab changes should be recorded here.

This project uses semantic pre-release versioning while the product is pre-launch:

- `0.x.y` for product milestones.
- `0.x.y-rc.n` for release candidates.
- Git tags use `v` prefixes, for example `v0.1.0`.

## [Unreleased]

### Added

- Engineering quality baseline for GitHub-first execution, CI gates, PR templates, issue templates, and release discipline.
- Guided workbench navigation, recoverable route errors, live setup progress, and shared query-state foundations.

### Changed

- CI now covers frontend formatting, Playwright smoke tests, technical handbook PDF generation, and SQLx migration checks.
- API timestamps now use RFC3339 strings while remaining compatible with legacy persisted timestamp arrays.
- The workbench shell now organizes CorpusLab around Build, Improve, Share, and Workspace workflows.

## [0.1.0] - Baseline Product Checkpoint

### Added

- CorpusLab workbench with ingestion, chunk preview, local embeddings, retrieval playground, trace debugger, Eval Lab, reports, settings, and marketing pages.
- Rust workspace with `apps/api`, `crates/core`, `crates/rag`, and `crates/storage`.
- React + Vite + TypeScript web app with strict typing, Vitest, Playwright, and CSS modules for newer surfaces.
- Postgres migrations for sources, documents, chunks, retrieval runs, embeddings, traces, evals, and Eval Lab.
- Engineering handbook and PDF generation workflow.
