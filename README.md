# RAG Debugger

RAG Debugger is a hybrid local/cloud debugging platform for retrieval-augmented generation systems. The first product wedge is diagnosis: make it obvious why a RAG answer failed, whether the cause is missing data, bad chunking, weak embeddings, poor ranking, prompt drift, or hallucination.

This repository starts as a production-grade scaffold: Rust workspace, Axum API, React TypeScript web app, shared domain contracts, storage interfaces, RAG engine boundaries, documentation, and CI.

## Repository Layout

```text
apps/
  api/       Axum API service with health/readiness routes and SQLx-ready state
  web/       React + Vite + TypeScript web app
crates/
  core/      Shared domain contracts and privacy model
  rag/       Ingestion, chunking, and retrieval interfaces
  storage/   Repository traits and Postgres adapter skeleton
docs/
  adr/       Architecture decision records
```

## Quick Start

Install Rust and Node.js 24 or newer, then:

```sh
cp .env.example .env
cd apps/web && npm install
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
- [Testing Guide](docs/testing.md)
- [Privacy and Security](docs/privacy-security.md)
- [Roadmap](docs/roadmap.md)
- [ADRs](docs/adr)

## Product Direction

The platform is designed around:

- Local-first data handling for raw documents.
- Shared team workflows for startup RAG debugging.
- Versioned traces, evals, prompts, indexes, and model configs.
- Future GPU/HPC workers for indexing, retrieval, embedding, reranking, and inference.

