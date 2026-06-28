# CorpusLab

CorpusLab is a corpus workbench and observability platform for retrieval-augmented generation teams. It helps engineers diagnose why evidence was or was not retrieved across PDFs, policies, product docs, support knowledge bases, research papers, contracts, technical specs, code docs, wikis, resumes, and enterprise document sets.

The current product slice includes file ingestion, structured document chunking, local embeddings, lexical/vector/hybrid retrieval, evidence summaries with citations, trace debugging, Eval Lab gates, CI-triggered eval runs, API keys, local auth/workspaces, corpus reports, safe runtime config, and a React workbench UI.

## Repository Layout

```text
apps/
  api/       Axum API service, config, ingestion, retrieval, traces, evals
  web/       React + Vite + TypeScript workbench UI
crates/
  core/      Shared contracts, reports, config, privacy model
  rag/       Extraction, chunking, document intelligence, embeddings, retrieval, evals
  storage/   Repository traits, memory store, Postgres adapter
docs/
  adr/       Architecture decision records
```

## Quick Start

Install Rust and Node.js 24 or newer, then:

```sh
cp .env.example .env
cd apps/web && npm install
```

Start local Postgres:

```sh
docker compose up -d postgres
```

Run the API:

```sh
cargo run -p rag-debugger-api
```

Run the web app:

```sh
cd apps/web && npm run dev
```

The default local login is seeded from `.env.example`:

```text
demo@corpuslab.ai
CorpusLab#2026
```

## Quality Checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/web && npm run typecheck && npm run lint && npm test && npm run build
```

## Documentation

- [Architecture](docs/architecture.md)
- [Development Guide](docs/development.md)
- [Engineering Quality](docs/engineering-quality.md)
- [Frontend Architecture](docs/frontend-architecture.md)
- [Documentation Maintenance](docs/doc-maintenance.md)
- [Marketing Experience](docs/marketing-experience.md)
- [File Ingestion](docs/file-ingestion.md)
- [Retrieval Playground](docs/retrieval-playground.md)
- [Trace Debugger](docs/trace-debugger.md)
- [Eval Lab](docs/eval-lab.md)
- [RAG Invariants](docs/rag-invariants.md)
- [Auth and Workspaces](docs/auth-and-workspaces.md)
- [CI Eval Workflows](docs/ci-eval-workflows.md)
- [Technical Handbook](docs/technical-handbook.md)
- [Testing Guide](docs/testing.md)
- [Privacy and Security](docs/privacy-security.md)
- [Privacy Review Checklist](docs/privacy-review-checklist.md)
- [Logging and Redaction](docs/logging-redaction.md)
- [Roadmap](docs/roadmap.md)
- [ADRs](docs/adr)
- [Changelog](CHANGELOG.md)

## Engineering Workflow

CorpusLab uses GitHub as the engineering source of truth. Track product work in GitHub Issues, land changes through pull requests into `main`, require CI before merge, and record milestone changes in `CHANGELOG.md`.

AI-agent and agent-assisted contributions must follow [AGENTS.md](AGENTS.md). Frontend ownership and file-placement rules are documented in the [Frontend Architecture Guide](docs/frontend-architecture.md).

Use the fast local gate while developing:

```sh
just check
```

Run one side of the repository when iterating on a focused change:

```sh
just rust-check
just web-check
```

Use the full release gate before baseline or milestone PRs:

```sh
just full-check
```

`just ci-check` is the explicit release-equivalent command behind `just full-check`.

## Product Direction

The platform is designed around:

- Local-first data handling for raw documents.
- Browser file ingestion with structured document chunking and persisted document/chunk metadata.
- Local embedding indexing, hybrid retrieval, evidence summaries, reports, and retrieval evals.
- Trace timelines, failure labels, and rerun comparisons for debugging RAG runs.
- Local auth, workspace membership, API keys, and CI gates for shared RAG quality reviews.
- Versioned traces, evals, prompts, indexes, and model configs.
- Future GPU/HPC workers for indexing, retrieval, embedding, reranking, and inference.

Generate the technical handbook PDF with:

```sh
just docs-pdf
```
