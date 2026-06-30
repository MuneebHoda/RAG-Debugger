# Logging And Redaction Policy

CorpusLab diagnostics must help operators investigate reliability without turning logs into a second corpus. This policy applies to API logs, workers, collectors, CI output, browser diagnostics, hosted telemetry, and support bundles.

## Allowed By Default

Logs may contain:

- trace, retrieval-run, eval-run, experiment, workspace, project, source, document, and chunk IDs;
- route templates, HTTP methods, status codes, durations, bounded result counts, and byte counts;
- retrieval mode, embedding readiness, model identifier, dimension, and configuration label;
- deterministic failure labels, gate status, aggregate metrics, and quality-flag counts;
- short checksum prefixes when required to correlate duplicate or stale derived data; and
- environment name, application version, storage backend kind, worker kind, and device class.

IDs must be opaque identifiers, not user email addresses, document paths, source names, or query-derived slugs. Log route templates such as `/traces/:trace_id`, not full URLs containing user-supplied query strings.

## Prohibited

Logs must not contain:

- raw files, extracted document text, chunk text, section text, or citation snippets;
- embedding vectors or serialized model inputs;
- retrieval query text, prompts, generated answers, evidence summaries, or eval question text;
- passwords, password hashes, session tokens, cookies, API keys, API-key hashes, authorization headers, database URLs, or provider credentials;
- complete request or response bodies for corpus, auth, retrieval, trace, eval, report, or export routes;
- SQL statements with bound customer values, stack dumps containing secrets, or unredacted external-provider payloads; or
- customer email addresses, names, document paths, source names, or workspace names unless a separately approved operational requirement exists.

Hashing a secret does not make it safe to log. Hashes used for session and API-key lookup remain credential-adjacent and are prohibited.

## Sensitive Queries

All query text is sensitive by default. A query can reproduce customer data, incident details, source-code names, health information, legal language, or credentials copied into a question. Product analytics should record query counts, lengths, modes, latencies, and outcomes, not query bodies.

An opt-in query capture feature requires a privacy ADR, workspace-level controls, retention and deletion behavior, role-gated access, visible user disclosure, export redaction, and tests proving the default remains off.

## Safe Correlation

Use structured fields with stable names:

```text
trace_id=<uuid> run_id=<uuid> document_id=<uuid> mode=hybrid hit_count=5 latency_ms=18 status=warning
```

Do not interpolate document text or query text into the event message. Keep event messages static and put approved metadata in structured fields. Limit high-cardinality fields to IDs needed for a concrete debugging workflow.

## Errors

Client-facing errors use the stable API error envelope and a safe message. Internal storage/provider details remain server-side, but server-side logging must still apply this policy. Record the error category, operation, safe IDs, and a generated correlation ID; do not log raw payloads or credentials.

Production panic and stack reporting must scrub environment values, headers, cookies, request bodies, SQL bind values, and provider payloads before transmission.

## Hosted Sync And Exports

Future hosted sync must use an allowlist, not a blocklist. The default sync payload may contain IDs, timestamps, modes, counts, aggregate scores, failure labels, gate outcomes, and explicitly approved configuration metadata.

Chunk text, query text, prompts, answers, citations, document paths, and trace span payloads remain excluded until the user explicitly enables a scoped sharing/export action. Before transfer:

1. remove credentials and headers;
2. replace raw content with IDs, counts, hashes, or bounded redacted excerpts;
3. enforce workspace/project ownership and destination policy;
4. show what will leave the local boundary;
5. apply retention and deletion settings; and
6. write an audit event containing metadata only.

## Current Audit

As of the current quality baseline, the API emits one structured startup event containing bind address, environment, and storage backend kind. The API does not log request bodies, query text, document/chunk text, credentials, headers, cookies, vectors, prompts, answers, or provider payloads.

Add request tracing only with explicit sensitive-header marking and route-template logging. Any new logging or telemetry path triggers the privacy checklist.
