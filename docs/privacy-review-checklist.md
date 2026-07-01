# Privacy Review Checklist

Privacy review is required whenever a change can alter where customer data moves, who can access it, how long it remains available, or what appears in diagnostics. The author completes this checklist before review; a reviewer other than the author confirms material privacy-boundary changes.

## Review Triggers

A pull request needs privacy review if it:

- uploads, syncs, exports, or shares raw document text;
- sends chunks, embeddings, queries, prompts, answers, citations, or traces to an external service;
- changes embedding, reranking, generation, OCR, analytics, or telemetry provider behavior;
- changes trace, report, eval, or corpus sync/export behavior;
- adds hosted, organization, workspace, collaboration, or report-sharing functionality;
- changes authentication, sessions, API keys, roles, retention, deletion, or access control;
- logs raw queries, chunks, prompts, answers, headers, cookies, credentials, or provider payloads; or
- changes the meaning of `PrivacyMode` or the local-first default.

## Data Movement

- [ ] Raw documents remain local by default.
- [ ] Uploaded binaries are discarded after extraction unless explicit retention is documented and approved.
- [ ] Raw chunks, vectors, queries, prompts, answers, and citations are not newly sent externally, or each destination and purpose is documented.
- [ ] External processing is explicit, scoped to the active workspace/project, and visible to the user.
- [ ] Exports and shares require an explicit user or CI action.
- [ ] Trace and report exports apply the documented redaction policy.
- [ ] Retention, deletion, and backup behavior is defined for every newly persisted data class.

## Secrets And Access

- [ ] Passwords, session tokens, cookies, API keys, authorization headers, provider secrets, and database URLs are never logged.
- [ ] API key and session secrets are stored only as one-way hashes; full API key secrets are shown once.
- [ ] New endpoints enforce the intended session, API-key scope, workspace, and role boundary.
- [ ] Error responses do not expose storage errors, SQL, credentials, internal paths, or provider payloads.
- [ ] Test fixtures and screenshots contain synthetic data only.

## Logs And Diagnostics

- [ ] Logs use opaque IDs, counts, durations, statuses, and approved short checksum prefixes.
- [ ] Query text is treated as sensitive unless a deliberate, documented opt-in permits capture.
- [ ] Raw document/chunk text, embedding vectors, prompt bodies, generated answers, and citation snippets are absent from logs.
- [ ] Debug logging cannot reveal secrets when enabled in production.
- [ ] New telemetry fields have a documented owner, purpose, retention period, and cardinality bound.

## Documentation And Decision Record

- [ ] `docs/privacy-security.md` remains accurate.
- [ ] `docs/logging-redaction.md` remains accurate.
- [ ] The relevant feature guide and technical handbook describe new data movement.
- [ ] An ADR records any hosted sync, external provider, retention, auth-provider, or privacy-boundary decision.
- [ ] `CHANGELOG.md` records user-visible privacy or security behavior.

Use `N/A` with a short reason in the pull request for checks that do not apply. A checked box means the author verified the behavior; it does not mean the feature has no privacy impact.

## Guided Demo Review Note

The guided demo uses only checked-in synthetic Markdown fixtures and local processing. Loading is an explicit authenticated action scoped to the active workspace. It persists ordinary extracted chunks and embeddings, never separate fixture binaries, and makes no external calls. Suggested query text is returned only to the authenticated client and is not placed in URLs. Audit reports default to `metadata_only`; `full_local_only` export remains blocked. Loading does not delete or reset customer data.

## Debugger Intelligence v2 Review Note

Structured diagnosis is derived locally from already persisted retrieval metadata. The diagnosis snapshot contains opaque IDs, ranks, scores, failure codes, counts, and deterministic remediation text; it does not copy raw queries, paths, section titles, snippets, chunk text, credentials, headers, cookies, or report bodies. No external call, new telemetry, sharing path, retention class, or authorization boundary is introduced. Metadata-only report tests verify that private source content cannot enter the diagnosis export, and full-local-only export remains blocked.
