# Architecture

RAG Debugger is a hybrid local/cloud system for diagnosing retrieval-augmented generation failures.

## Components

- **Web app:** React TypeScript debugger workbench for traces, sources, evals, and privacy controls.
- **API service:** Axum backend for health checks, project metadata, team workflows, trace summaries, and eval results.
- **Core crate:** Shared domain contracts for projects, sources, chunks, traces, retrieval runs, evals, models, and privacy mode.
- **RAG crate:** Ingestion, chunking, and retrieval interfaces. Implementations are intentionally replaceable.
- **Storage crate:** Repository traits and Postgres adapter skeleton.
- **Local collector:** Future local process that reads raw documents, builds indexes, runs local traces, and syncs approved summaries.
- **Workers:** Future local or remote jobs for parsing, embedding, indexing, retrieval, reranking, generation, and eval scoring.

## Privacy Boundary

Raw documents are local by default. Cloud services should receive project metadata, redacted traces, metrics, and explicitly approved snippets only. `PrivacyMode` is part of the core model so every sync path must make a deliberate choice.

## API Boundary

All product APIs are versioned under `/api/v1`. Public health probes remain at `/healthz` and `/readyz` for deployment compatibility.

## Storage Direction

Postgres is the target metadata store. Vector/index storage can begin local and evolve toward LanceDB, Postgres extensions, or GPU-backed services as benchmarks justify it.

## GPU/HPC Direction

The current local machine path should support Apple Silicon experiments through Metal-friendly tooling. The long-term worker model should support CUDA/NVIDIA jobs for high-throughput embedding, index builds, vector search, reranking, and inference.

