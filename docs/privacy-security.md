# Privacy and Security

RAG Debugger handles sensitive traces, prompts, retrieved context, and source documents. Privacy is a product feature, not a later compliance task.

## Defaults

- Raw documents stay local by default.
- Cloud sync should prefer redacted traces, metrics, configs, and eval summaries.
- Snippet sync must be explicit and project-scoped.
- Secrets must come from environment variables or a secret manager, never committed files.
- Browser-uploaded binaries are not persisted in v1 file ingestion.
- Local embeddings are derived and stored locally; no hosted embedding API is called in v1.
- `GET /api/v1/config` exposes safe runtime config only. Database URLs, secret keys, and deployment internals stay server-side.

## Data Classes

- **Raw documents:** customer-owned, local by default.
- **Chunks:** derived from raw documents, including section titles, split metadata, quality flags, and evidence hints, treated as sensitive.
- **Embeddings:** derived from chunks, stored locally, and treated as sensitive because they can leak information about source text.
- **Retrieval queries and evidence summaries:** local by default; citations and snippets are treated as sensitive derived document data.
- **Uploaded binaries:** processed in memory for v1 ingestion, then discarded.
- **Traces:** sensitive because prompts and retrieved context may contain private data.
- **Metrics:** usually safe after aggregation, but still project-owned.
- **Eval datasets:** sensitive when derived from real user questions or internal docs.

## Engineering Requirements

- Every sync path must check `PrivacyMode`.
- Logs must follow [`docs/logging-redaction.md`](logging-redaction.md): raw document/chunk text, queries, prompts, answers, vectors, credentials, headers, and cookies are prohibited.
- Safe diagnostics use opaque IDs, counts, statuses, durations, failure labels, aggregate metrics, and approved short checksum prefixes.
- Upload handlers must enforce file count, per-file size, total request size, and supported type limits.
- Workbench APIs require local authenticated sessions in development.
- CI automation uses workspace-scoped API keys. Full secrets are shown once, stored only as hashes, and can be revoked.
- Future hosted APIs should add invitations, SSO/SAML, SCIM, deeper RBAC, audit logging, and per-workspace retention settings.
- Any export path must preserve project ownership and deletion semantics.
- Report sharing must support redaction before it becomes a hosted/team feature.

Changes that move data, add external providers, alter auth/retention/export behavior, or add telemetry must complete the [`Privacy Review Checklist`](privacy-review-checklist.md). Hosted sync and external model-provider boundaries require an ADR.
