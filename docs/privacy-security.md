# Privacy and Security

RAG Debugger handles sensitive traces, prompts, retrieved context, and source documents. Privacy is a product feature, not a later compliance task.

## Defaults

- Raw documents stay local by default.
- Cloud sync should prefer redacted traces, metrics, configs, and eval summaries.
- Snippet sync must be explicit and project-scoped.
- Secrets must come from environment variables or a secret manager, never committed files.

## Data Classes

- **Raw documents:** customer-owned, local by default.
- **Chunks:** derived from raw documents, treated as sensitive.
- **Traces:** sensitive because prompts and retrieved context may contain private data.
- **Metrics:** usually safe after aggregation, but still project-owned.
- **Eval datasets:** sensitive when derived from real user questions or internal docs.

## Engineering Requirements

- Every sync path must check `PrivacyMode`.
- Logs must avoid raw document text and full prompts.
- Future hosted APIs should include authentication, authorization, audit logging, and per-project retention settings.
- Any export path must preserve project ownership and deletion semantics.

