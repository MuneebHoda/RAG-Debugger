# CorpusLab

CorpusLab is a corpus workbench and observability platform for retrieval-augmented generation teams. It helps engineers diagnose why evidence was or was not retrieved across PDFs, policies, product docs, support knowledge bases, research papers, contracts, technical specs, code docs, wikis, resumes, and enterprise document sets.

The current product slice includes file ingestion, structured document chunking, local embeddings, lexical/vector/hybrid retrieval, evidence summaries with citations, trace debugging, eval cases, corpus reports, safe runtime config, and a React workbench UI.

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
- [File Ingestion](docs/file-ingestion.md)
- [Retrieval Playground](docs/retrieval-playground.md)
- [Trace Debugger](docs/trace-debugger.md)
- [Eval Lab](docs/eval-lab.md)
- [Technical Handbook](docs/technical-handbook.md)
- [Testing Guide](docs/testing.md)
- [Privacy and Security](docs/privacy-security.md)
- [Roadmap](docs/roadmap.md)
- [ADRs](docs/adr)
- [Changelog](CHANGELOG.md)

## Engineering Workflow

CorpusLab uses GitHub as the engineering source of truth. Track product work in GitHub Issues, land changes through pull requests into `main`, require CI before merge, and record milestone changes in `CHANGELOG.md`.

Use the fast local gate while developing:

```sh
just check
```

Use the full release gate before baseline or milestone PRs:

```sh
just full-check
```

## Product Direction

The platform is designed around:

- Local-first data handling for raw documents.
- Browser file ingestion with structured document chunking and persisted document/chunk metadata.
- Local embedding indexing, hybrid retrieval, evidence summaries, reports, and retrieval evals.
- Trace timelines, failure labels, and rerun comparisons for debugging RAG runs.
- Shared team workflows for corpus debugging and RAG quality reviews.
- Versioned traces, evals, prompts, indexes, and model configs.
- Future GPU/HPC workers for indexing, retrieval, embedding, reranking, and inference.

Generate the technical handbook PDF with:

```sh
just docs-pdf
```
