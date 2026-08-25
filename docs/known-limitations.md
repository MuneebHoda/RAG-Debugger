# Known Limitations

This document describes the current CorpusLab pre-release boundary for design partners. It is intentionally direct: planned work is not presented as completed behavior.

## Operating Model

- CorpusLab is a local-first MVP and currently runs in the partner's environment.
- Hosted/private-alpha access is not available yet. Hosted onboarding is deferred to issue #29.
- There is no public SaaS signup, billing, paid-plan enforcement, deployment service, service-level agreement, or production support promise.
- Local signup creates a local organization/workspace in the configured instance; it is not a public hosted account.
- The workbench exposes a workspace-owned default project and a deterministic guided-demo project, but no general project creation or switching UI.
- Selective deletion is incomplete. Eval cases and API keys can be removed, but sources, traces, experiments, and reports do not yet have general workbench deletion controls.

## RAG And Corpus Processing

- Browser ingestion supports text, Markdown, HTML, and PDFs with embedded text. It does not provide OCR for scanned PDFs.
- Original uploaded binaries are processed in memory and discarded; extracted text and derived chunks remain in the configured database.
- The current embedding provider is a local hash baseline. It is suitable for deterministic product validation, not a claim of production embedding quality.
- Indexing and several product workflows are synchronous. There is no background worker/queue system for large corpora.
- No GPU/HPC worker, ANN service, hosted reranker, or external model judge is included.
- CorpusLab diagnoses retrieval and builds deterministic evidence summaries; it is not an answer-generation or chatbot runtime.

## Trace Integration

- External integrations are limited to the versioned native JSON contract and OTLP/HTTP protobuf semantic mapping.
- OTLP supports uncompressed HTTP/protobuf only. OTLP JSON, gzip, and gRPC are unsupported.
- There are no published CorpusLab SDK packages or maintained LangChain/LlamaIndex/framework adapters.
- OTLP is always `metadata_only`; content attributes are stripped before storage.
- Semantic mapping is bounded and intentionally ignores unknown attributes. Unsupported operations may remain visible as partially mapped spans.
- Imported traces are observational snapshots and cannot be rerun inside CorpusLab.
- CorpusLab is not a general-purpose APM platform: it does not provide arbitrary telemetry retention, service maps, infrastructure monitoring, logs, metrics, or distributed-tracing analytics.

## Privacy And Sharing

- Privacy modes require deliberate selection and project-policy review; stronger retention must never be chosen merely to make a demo easier.
- `metadata_only` imports retain structural metadata but no reproducible query, so they cannot create imported Eval cases.
- `snippets_allowed` imports retain bounded approved evidence snippets but no query, so they cannot create imported Eval cases.
- `full_local_only` imported data may create a provenance-locked local Eval case, but cannot enter CI, audit reports, Markdown, clipboard, download, or API export paths.
- OTLP cannot retain snippets or full-local content in v1.
- There are no public report links, hosted sharing, team comments, redaction preview workflow, or PDF report export.
- Markdown report export is available only when the report privacy mode permits it.
- Local-first operation reduces data movement but is not a compliance certification. CorpusLab makes no SOC 2, ISO 27001, HIPAA, GDPR, or other certification claim.

## Authentication And Administration

- Authentication is a local Postgres-backed provider with opaque sessions and workspace memberships.
- There is no enterprise SSO/SAML, SCIM, invitation flow, complex RBAC, service-account administration, or hosted tenant administration.
- API keys are scoped to the current workspace and supported automation purpose; they are shown once and stored only as hashes.
- General audit-event history, configurable retention, backup management, and disaster-recovery automation are not implemented.

## Eval Lab And CI

- Eval quality depends on the expected evidence selected by the user. CorpusLab does not supply a human or LLM relevance judge.
- Regression comparison requires a compatible earlier experiment with the same dataset, `top_k`, and sorted retrieval-mode set.
- The checked-in CI workflow evaluates the CorpusLab instance, corpus, index, and retrieval configuration reachable at `CORPUSLAB_API_URL`.
- The CI workflow does not automatically build or execute a pull request's changed RAG application. Candidate-specific evaluation requires a candidate-specific reachable target.
- A cloud-hosted CI runner cannot reach a developer machine at `127.0.0.1`; private/local use normally requires a self-hosted runner or another explicitly reachable private address.
- Datasets containing `full_local_only` imported provenance are rejected from CI.

## Scale And Production Readiness

- Postgres is the scalable persistence/search path; MemoryStore is for tests and focused local development.
- Published corpus-size, indexing-throughput, latency, and concurrency benchmarks do not yet exist.
- The product has not completed a hosted production-readiness program, penetration test, enterprise compliance review, or broad design-partner validation.
- Deployment, observability, backups, upgrades, TLS termination, networking, and host hardening remain the operator's responsibility.
- The repository is public pre-release software, APIs may evolve before a stable release, and the repository currently has no software license.

## Planned, Not Available

The near-term sequence is to validate the local product loop with 3 to 5 RAG builders, use that evidence to refine issue #29 and deployment issues #102-#108, and only then build a private hosted alpha. Public SaaS, billing, enterprise controls, broad integrations, hosted sharing, and production infrastructure must not be assumed from roadmap language.

Use [Design-Partner Onboarding](design-partner-onboarding.md) for the supported local path and [Roadmap](roadmap.md) for directional future work.
