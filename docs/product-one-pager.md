# CorpusLab Product One-Pager

## Product

**CorpusLab is a RAG debugging, evaluation, and audit platform for teams building document-based AI systems.**

**CI and observability for RAG quality.**

## The Problem

A plausible RAG answer can still be grounded in the wrong chunk, weak evidence, stale embeddings, or a retrieval configuration that quietly regressed. Aggregate scores and generic traces rarely tell an engineer which evidence caused the failure, whether the answer was supportable, or whether a proposed fix helped known cases.

## Target User

CorpusLab is for developers, ML engineers, AI platform teams, and technical consultants who own a document-based AI system and need reproducible evidence about retrieval quality.

It is not a chatbot builder, PDF Q&A product, vector database, LangChain or LlamaIndex replacement, generic OpenTelemetry/APM platform, or hosted SaaS.

## Product Promise

CorpusLab connects one evidence lineage across the RAG quality loop:

> Observe a real RAG failure -> inspect its retrieval evidence -> diagnose it -> preserve it as an evaluation -> compare fixes -> prevent regressions through CI.

## Current Workflow

1. **Build:** ingest approved local documents, inspect extraction/chunk quality, and index local embeddings.
2. **Test:** run lexical, vector, or hybrid retrieval and inspect ranked evidence, score signals, citations, and answerability.
3. **Debug:** save a trace with deterministic failure labels, affected evidence, recommendations, timeline, and rerun comparison.
4. **Measure:** convert known failures into expected-evidence Eval Lab cases and compare experiments over time.
5. **Gate:** call the Eval Lab CI endpoint and block a change when a deterministic release gate fails.
6. **Audit:** create a privacy-classified report with evidence lineage and controlled Markdown export.

## Current Capabilities

- Local text, Markdown, HTML, and embedded-text PDF ingestion.
- Structured and whitespace chunking with quality flags and document profiles.
- Local hash embeddings plus lexical, vector, and hybrid retrieval.
- Deterministic answerability and retrieval diagnosis without external LLM calls.
- Trace Debugger with ranked evidence, score explanations, citations, failure labels, timelines, and comparisons.
- Eval Lab datasets, exact-chunk/document expectations, multi-mode experiments, regression history, and gates.
- Workspace-scoped API keys for CI evaluation and native/OTLP trace ingestion.
- Audit reports from traces, experiments, and CI runs with privacy-aware Markdown export.
- Memory and Postgres storage adapters with workspace-isolation tests.

## Why This Is More Than A Tracing Dashboard

A tracing dashboard shows that operations happened. CorpusLab also evaluates the RAG-specific evidence path:

- Did the chunk body directly support the question?
- Why did each chunk rank where it did?
- Was expected evidence missing, weak, duplicated, stale, heading-only, or in the wrong chunk?
- Did a rerun or experiment improve known cases?
- Should the change pass a release gate?
- Can the diagnosis be shared without exposing raw documents?

The same deterministic diagnosis model feeds retrieval, traces, Eval Lab, CI gates, and reports so those surfaces do not invent conflicting explanations.

## Integration

- **Native JSON:** a bounded versioned contract at `POST /api/v1/traces/ingest` for explicit RAG evidence, spans, labels, configuration, and privacy mode.
- **OTLP/HTTP:** uncompressed protobuf at `POST /api/v1/otel/v1/traces`, using mapped CorpusLab, OpenInference, and OpenTelemetry GenAI semantics.
- **CI:** a workspace-scoped API key calls `POST /api/v1/eval-lab/ci/runs` against a reachable CorpusLab instance.

Checked-in native, Python SDK, Collector, and GitHub Actions examples use environment variables and synthetic data. No public SDK package is available yet.

## Privacy Model

CorpusLab currently runs inside the partner's environment. Uploaded binaries are processed in memory and discarded; extracted text, chunks, embeddings, traces, evals, and reports remain in the configured local/private database.

Imported traces have three deliberate modes:

- `metadata_only`: structural metadata only; no reproducible imported Eval case.
- `snippets_allowed`: bounded approved evidence snippets; no reproducible imported Eval case.
- `full_local_only`: local content may support a provenance-locked Eval case, but cannot enter CI, reports, Markdown, copy, or export.

OTLP is always metadata-only. Report creation separately defaults to metadata-only, and full-local reports cannot be exported.

## Design-Partner Offering

The current offer is a controlled local-first validation with 3 to 5 RAG builders or teams:

- CorpusLab runs on the partner's machine or private development environment.
- Sensitive data does not need to leave that machine.
- The partner tests a guided synthetic flow before any approved local corpus.
- Feedback focuses on installation, time to first useful trace, diagnosis quality, Eval Lab, CI gates, reports, privacy, and missing integrations.

This is not a public SaaS launch. Hosted access and onboarding are deferred to issue #29.

## Honest Limitations

CorpusLab does not currently provide hosted access, public SaaS signup, billing, enterprise SSO, complex RBAC, OTLP/gRPC, OTLP JSON, compressed OTLP, public SDK packages, broad framework adapters, public report links, PDF report export, OCR, background worker infrastructure, or general-purpose APM functionality.

CI evaluates the configured CorpusLab instance and dataset; it does not automatically run a pull request's changed RAG application. See [Known Limitations](known-limitations.md) for the complete current boundary.

## Near-Term Roadmap

1. Validate the local-first workflow with 3 to 5 real RAG builders.
2. Use their evidence to refine the hosted-alpha requirements in issue #29 and deployment issues #102-#108.
3. Build the private hosted alpha only after the local product loop and privacy expectations are understood.

This roadmap is planned work, not a claim of completed hosted capability.

## Feedback Requested

- Which retrieval failures consume the most engineering time?
- How quickly can a developer reach a useful imported or saved trace?
- Does the diagnosis identify the evidence and next action clearly?
- Can real failures be converted into useful Eval Lab cases?
- Would the deterministic gate fit the team's CI process?
- Is the audit report useful for engineering or client review?
- What data-residency rules and integrations are required for repeated use or payment?

Start with [Design-Partner Onboarding](design-partner-onboarding.md), follow the [Demo Script](demo-script.md), and record sanitized findings in [Design-Partner Feedback](design-partner-feedback.md).
