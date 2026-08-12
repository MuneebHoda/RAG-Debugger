# ADR 0007: Local Native and OTLP/HTTP Trace Ingestion

- Status: Accepted
- Date: 2026-08-12

## Context

CorpusLab previously inspected only retrieval runs created inside its workbench. External RAG applications need a standards-compatible path without making protocol-generated types part of the domain or weakening local privacy.

## Decision

Provide versioned native JSON as the high-fidelity contract and authenticated OTLP/HTTP protobuf as the compatibility path. Generated OpenTelemetry types remain in the Axum boundary and map into additive core trace contracts. Workspace authority always comes from existing session or hashed, scoped API-key authentication; explicit projects are verified inside it.

Privacy is enforced after bounded parsing and before persistence or secondary use. OTLP is metadata-only until the project model has a separate explicit trusted retention setting. Imported identities merge atomically by workspace, project, source, and external trace ID. PostgreSQL uses an additive partial unique index and transaction lock; memory uses its existing mutex.

The API adds `prost` and `opentelemetry-proto` with only generated-message and trace features. Hand-written messages would duplicate a standard; a gRPC stack and JSON feature are unnecessary for v1. These Apache-2.0-compatible dependencies add protobuf decoding only to the API binary, make no network calls, and do not change the web bundle or local-first data boundary. The pinned Python SDK/exporter are example-only dependencies.

The Python example pins `opentelemetry-sdk` and `opentelemetry-exporter-otlp-proto-http` 1.41.1 so users can produce a standards-compatible payload without a CorpusLab SDK. They are installed only in the user's example environment, not the application or CI runtime; use the same Apache-2.0 ecosystem, contact only the configured local endpoint, and have no web-bundle or server-binary impact. Raw HTTP/protobuf construction was rejected as harder to audit and maintain. The native shell example remains the dependency-free alternative, and future version changes should receive normal license, advisory, and local-endpoint review.

## Consequences

Applications can send retry-safe native or standard OTLP/HTTP protobuf traces. OTLP JSON, compression, gRPC, arbitrary attribute storage, framework adapters, and public SDK packages remain non-goals. Mapping is versioned and reports `partially_mapped` whenever privacy or missing RAG semantics limits diagnosis.
