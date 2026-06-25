# Contributing

RAG Debugger is being built as a production-grade systems project. Keep changes small, typed, tested, and documented.

## Local Setup

1. Install Rust with `rustup`.
2. Install Node.js 24 or newer.
3. Copy `.env.example` to `.env` and adjust values if needed.
4. Run `npm install` in `apps/web`.

## Quality Bar

Before opening a pull request, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/web && npm run typecheck && npm run lint && npm test && npm run build
```

For baseline, release, or migration-heavy pull requests, run:

```sh
just full-check
```

## GitHub Workflow

- Create a GitHub issue before starting product, refactor, bug, security, UI, or performance work.
- Create a branch from `main`; do not commit directly to `main`.
- Open a pull request and fill out the PR template.
- Use conventional PR titles: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `perf:`, or `security:`.
- Squash merge after CI passes.
- Update `CHANGELOG.md` for user-facing, API, storage, release, or workflow changes.

## Engineering Principles

- Prefer explicit domain types over loose JSON maps.
- Keep raw customer documents local by default.
- Add an ADR when a decision changes architecture, storage, privacy posture, or public API shape.
- Keep APIs versioned under `/api/v1`.
- Treat traces and eval datasets as high-value, sensitive product data.
- Split large files before they become ownership bottlenecks. If a PR grows a major file, either split it or create a linked cleanup issue.
