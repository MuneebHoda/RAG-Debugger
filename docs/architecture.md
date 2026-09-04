# Architecture

RAG Debugger is a hybrid corpus observability system for diagnosing retrieval-augmented generation failures across arbitrary document sets.

## Components

- **Web app:** React TypeScript workbench for Overview, Sources, Retrieval, Traces, Evals, Reports, and Settings.
- **Public experience:** Lazy-loaded React marketing routes with an editorial landing narrative, typed interactive examples, responsive product imagery, and reduced-motion support.
- **API service:** Axum backend for health checks, runtime config, local auth, workspaces, API keys, ingestion, embedding status/indexing, retrieval, traces, evals, CI gates, and reports.
- **Core crate:** Shared domain contracts for projects, sources, documents, chunks, traces, retrieval runs, evals, reports, config, models, and privacy mode.
- **RAG crate:** File text extraction, structured and whitespace chunking, document intelligence, local embedding generation, hybrid retrieval, trace construction, eval scoring, ingestion, and retrieval interfaces. Implementations are intentionally replaceable.
- **Storage crate:** Bounded repository traits for health, projects, sources, documents, evidence lookup, embeddings, retrieval, traces, evals, auth, CI evals, and audit reports, plus Postgres and in-memory adapters.
- **Local collector:** Future local process that reads raw documents, builds indexes, runs local traces, and syncs approved summaries.
- **Workers:** Future local or remote jobs for parsing, embedding, indexing, retrieval, reranking, generation, and eval scoring.

## Privacy Boundary

Raw documents are local by default. Cloud services should receive project metadata, redacted traces, metrics, and explicitly approved snippets only. `PrivacyMode` is part of the core model so every sync path must make a deliberate choice.

For browser upload ingestion, original file bytes are processed in memory and not persisted. Extracted chunk text is stored in local Postgres so the user can refresh the Sources page and inspect chunks again.

Local embeddings are derived from persisted chunk text and stored in Postgres. They stay inside the local database in v1 and are not sent to hosted services.

## API Boundary

All product APIs are versioned under `/api/v1`. Public health probes remain at `/healthz` and `/readyz` for deployment compatibility.

`apps/api/src/http/mod.rs` is the handler-module index. `apps/api/src/http/routing.rs` owns public/protected route composition, session middleware, request-size limits, and CORS. Handler implementations remain in bounded HTTP modules, and route paths must not change during routing refactors.

Failed API requests use one stable JSON envelope:

```json
{
  "error": {
    "code": "not_found",
    "message": "not found: trace"
  }
}
```

Clients may present `error.message` and use `error.code` for typed behavior. Internal and storage failures return generic messages so database or infrastructure details are not exposed; server-side diagnostics remain in telemetry.

Current ingestion APIs:

- `GET /api/v1/config`
- `POST /api/v1/auth/signup`
- `POST /api/v1/auth/login`
- `POST /api/v1/auth/logout`
- `GET /api/v1/auth/me`
- `GET /api/v1/workspaces/current`
- `GET /api/v1/demo`
- `POST /api/v1/demo/load`
- `GET /api/v1/api-keys`
- `POST /api/v1/api-keys`
- `DELETE /api/v1/api-keys/:api_key_id`
- `GET /api/v1/projects/current`
- `POST /api/v1/traces/ingest`
- `POST /api/v1/otel/v1/traces`
- `POST /api/v1/sources/files`
- `GET /api/v1/sources`
- `GET /api/v1/documents/:document_id/chunks`
- `GET /api/v1/embeddings/status`
- `POST /api/v1/embeddings/index`
- `POST /api/v1/retrieval/query`
- `GET /api/v1/retrieval/evals`
- `POST /api/v1/retrieval/evals`
- `POST /api/v1/retrieval/evals/run`
- `GET /api/v1/eval-lab/datasets`
- `POST /api/v1/eval-lab/datasets`
- `GET /api/v1/eval-lab/datasets/:dataset_id`
- `POST /api/v1/eval-lab/evidence/query`
- `POST /api/v1/eval-lab/datasets/:dataset_id/cases`
- `PATCH /api/v1/eval-lab/cases/:case_id`
- `DELETE /api/v1/eval-lab/cases/:case_id`
- `POST /api/v1/eval-lab/experiments`
- `GET /api/v1/eval-lab/experiments`
- `GET /api/v1/eval-lab/experiments/:experiment_id`
- `POST /api/v1/eval-lab/experiments/:experiment_id/compare`
- `POST /api/v1/eval-lab/ci/runs`
- `GET /api/v1/eval-lab/ci/runs`
- `GET /api/v1/eval-lab/ci/runs/:run_id`
- `GET /api/v1/eval-lab/ci/runs/:run_id/report`
- `GET /api/v1/traces`
- `GET /api/v1/traces/:trace_id`
- `POST /api/v1/traces/from-retrieval-run`
- `POST /api/v1/traces/:trace_id/rerun`
- `GET /api/v1/reports`
- `GET /api/v1/reports/:report_id`
- `POST /api/v1/reports/from-trace`
- `POST /api/v1/reports/from-experiment`
- `POST /api/v1/reports/from-ci-run`
- `GET /api/v1/reports/:report_id/export.md`

Uploads default to `structured` chunking. The API also accepts `chunking_strategy=smart_sections` as a legacy alias and `chunking_strategy=whitespace` for baseline/debug runs. Chunk responses include the strategy, detected section title, split reason, token count, byte range, checksum, quality flags, duplicate status, text density, and evidence hints so the UI can explain why each chunk exists.

Retrieval queries search all indexed documents by default and can filter by source or document. V1 retrieval is local-only: lexical scoring, local vector similarity, hybrid score blending, phrase boosts, section/path boosts, evidence summaries, quality flags, evidence strength, duplicate suppression, and no hosted model calls.

Embedding indexing is synchronous in v1. The API reports missing or stale embeddings explicitly so hybrid/vector retrieval never silently degrades to lexical-only behavior.

Trace debugging wraps retrieval responses into inspectable timelines. A trace stores the query, spans, ranked evidence, citations, deterministic failure labels, and rerun comparisons. Reruns reuse the same local retrieval engine with different mode or `top_k` settings.

The guided demo is an orchestration layer over existing bounded contexts, not a parallel data model. Its fixtures pass through the shared API ingestion preparation service, then persist as normal workspace-owned project/source/document/chunk records. `DemoRepository` owns deterministic upserts and source-specific progress lookups. SHA-256-derived IDs and a versioned source marker make repeated loads repairable without a migration.

Eval Lab is the release-readiness layer. It stores datasets, expected-evidence cases, cross-mode experiments, deterministic failure labels, and pass/fail gates. Retrieval and trace workflows can save observed evidence directly into a dataset so real debugging sessions become regression coverage.

Manual and CI experiments share one runner that freezes workspace-scoped source/document/chunk/embedding inputs through one repository snapshot operation and stores versioned, canonical provenance with the experiment. MemoryStore clones under one mutex; Postgres reads under one read-only `REPEATABLE READ` transaction. Automatic regression baselines require identical identity fingerprints; explicit cross-configuration comparisons remain available with typed warnings. Provenance is additive in the existing experiment JSON, hashes privacy-sensitive retrieval inputs instead of copying them, and leaves legacy experiments readable as `legacy_unknown`. See [ADR 0008](adr/0008-immutable-eval-provenance.md).

`EvidenceRepository` separates direct expected-evidence resolution from bounded candidate search. Every operation requires the authenticated `WorkspaceId`; ownership is enforced by the MemoryStore/Postgres adapter before metadata is returned. The API caps submitted ID work before repository access, performs fixed direct-resolution calls, and uses typed `Browse`, `ExactId`, or `Text` search modes rather than storage-specific string interpretation. Requested IDs retain stable request order and sit outside candidate limits. Postgres joins evidence through `documents → sources → projects` and requires the project workspace; MemoryStore applies the same check through its project ownership index. Cross-workspace IDs are indistinguishable from nonexistent IDs. Evidence responses expose 280-character UTF-8-safe previews rather than complete chunk bodies.

The same repository boundary owns runtime RAG state. Retrieval runs and traces persist `workspace_id`; trace writes additionally verify the trace project and optional source run. Embedding status, candidate loading, and writes derive ownership through `chunk → source → project`, and a mixed or foreign write set is rejected before mutation. Trace lists, details, reruns, report creation, guided-demo progress, and Overview metrics all pass the authenticated workspace into storage. Foreign and nonexistent run or trace IDs use the same sanitized `404` behavior.

Eval datasets, cases, legacy runs, experiments, CI runs, and related overview/report reads use the same repository-enforced workspace boundary. Dataset ownership is persisted directly; cases and experiments inherit it through their dataset. New experiment writes also validate provenance workspace/project ownership and reject duplicate IDs so snapshots are append-only. The workspace-isolation migration backfills records only when ownership is uniquely attributable or the installation has exactly one workspace. Ambiguous multi-workspace legacy rows remain unowned and invisible until an operator deliberately repairs them.

Local auth protects workbench APIs with opaque HttpOnly session cookies. Workspace-scoped API keys authorize CI automation and are stored only as hashes. The local auth provider owns signup/login/session validation today; an external provider can replace that boundary later.

## Storage Direction

Postgres stores organizations, workspaces, users, memberships, sessions, API keys, projects, sources, ingestion runs, documents, chunks, chunking metadata, document profile metadata, chunk quality metadata, local chunk embeddings, workspace-owned retrieval playground runs, retrieval hits, workspace-owned trace debugger records, trace rerun experiments, retrieval eval datasets, eval cases, legacy eval run results, Eval Lab experiments, CI eval runs, workspace-scoped audit report snapshots, and gate outcomes. Legacy ownership is backfilled only when unambiguous or when exactly one workspace exists; ambiguous multi-workspace runtime records remain quarantined. The semantic retrieval migration stores vectors as local Postgres arrays for the first debugger loop; vector/index storage can evolve toward pgvector, LanceDB, or GPU-backed services as benchmarks justify it.

## Hosted Product Direction

The approved small private-alpha topology is documented in [Private-Alpha Deployment Architecture](deployment-architecture.md) and [ADR 0010](adr/0010-private-alpha-deployment.md). It uses immutable GitHub-built artifacts, Cloudflare Pages/Access/Tunnel, a private image-backed Render API, and isolated Render Postgres. Staging and production are separate trust and data boundaries; production data never enters pull-request or staging environments by default.

The repository contains the hardened production API image, static web artifact, public runtime-config boundary, explicit packaged migration command, and opt-in production-parity Compose stack documented in [Production Artifacts](production-artifacts.md). Provider resources, publishing/deployment workflows, hosted observability, and recovery runbooks remain owned by issues #104–#108. Hosted `staging` and `production` configuration fails before startup when origins, TLS database configuration, secure cookie namespace, release identity, local embedding boundary, or upload limits are unsafe.

The codebase now has the first hosted/team foundation without billing, invitations, SSO, or SCIM:

- Organizations contain workspaces/projects.
- Users and roles govern access to sources, retrieval runs, evals, and reports.
- API keys allow collectors and CI jobs to upload traces or run evals.
- A local collector can keep raw documents inside private networks while syncing approved metadata and reports.
- A hosted control plane can manage dashboards, report sharing, audit events, and collaboration.
- Worker queues can run ingestion, OCR, embeddings, reranking, evals, and report generation.

## GPU/HPC Direction

The current local machine path should support Apple Silicon experiments through Metal-friendly tooling. The long-term worker model should support CUDA/NVIDIA jobs for high-throughput embedding, index builds, vector search, reranking, and inference.

## External Trace Boundary

Native JSON and OTLP/HTTP protobuf receivers are trust boundaries. Generated OTLP types exist only in `apps/api`; they map into versioned core contracts before reaching RAG behavior or storage. Both receivers authenticate, resolve the workspace from trusted state, verify the explicit project, enforce bounds and privacy, then atomically merge by external identity. Full-local Eval conversion derives immutable provenance from a workspace-owned source trace and propagates it through experiment results so CI and report boundaries can reject outward use. See [ADR 0007](adr/0007-local-trace-ingestion.md).
