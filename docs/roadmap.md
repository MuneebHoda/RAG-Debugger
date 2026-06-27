# Roadmap

## Phase 1: Scaffold

- Rust workspace and shared domain contracts.
- Axum API with health/readiness routes.
- React TypeScript app shell.
- Engineering handbook and CI.

## Phase 2: Corpus Workbench

- Browser file ingestion with `.txt`, `.md`, `.html`, and embedded-text `.pdf` support.
- Text extraction, structured chunking, whitespace fallback, document profile detection, chunk quality flags, and persisted chunk preview.
- Local retrieval playground with lexical, vector, and hybrid scoring plus evidence summaries.
- Local embedding indexing and vector storage adapter.
- Retrieval eval cases with recall/precision measurement.
- Overview, Sources, Retrieval, Evals, Reports, and Settings workbench pages.

## Phase 3: Debugger Workbench

- Trace timeline.
- Retrieved chunk inspection.
- Failure labels.
- Retrieval deduplication, evidence strength, normalized score explanations, weak-evidence warnings, and report generation.
- Rerun with changed chunking, embedding, ranking, top-k, and prompt settings.

## Phase 4: Eval Lab

- Golden datasets and editable expected-evidence cases.
- Cross-mode experiments across lexical, vector, and hybrid retrieval.
- Retrieval recall, precision, MRR, citation coverage, weak-evidence, missing-embedding, and latency metrics.
- Deterministic failure labels for missing evidence, wrong chunk, low precision, weak evidence, missing embeddings, heading-only evidence, and duplicate evidence.
- Release gates that surface pass/fail status in Mission Control.

## Phase 5: Release And CI Workflows

- API keys for CI eval runs.
- Branch/config snapshots for retrieval changes.
- Historical experiment trends and regression diffs.
- Report export from failed gates.
- Team comments on failed cases and traces.

Status: local auth, workspaces, hashed API keys, and CI Eval Lab runs are implemented as the first hosted foundation. Next work should harden permissions, report exports, and CI ergonomics before adding hosted billing.

## Phase 6: Hybrid Team Product

- Organizations, workspaces, users, roles, and API keys.
- Local collector.
- Hosted project/team dashboard.
- Redacted trace sync.
- Shareable debug reports, audit events, and report redaction.

## Phase 7: GPU/HPC Workers

- Apple Silicon local acceleration experiments.
- Stronger ONNX/GPU embedding backend behind the existing local provider boundary.
- CUDA remote workers for embedding, index builds, search, reranking, and inference.
- Benchmarks across CPU, local GPU, and cloud GPU.
