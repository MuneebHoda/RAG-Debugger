# Local trace ingestion

CorpusLab accepts RAG traces from external applications through a native JSON contract or authenticated OTLP/HTTP protobuf. Both paths derive the workspace from authentication, require an explicit workspace-owned project, enforce privacy before storage, and require no hosted service or external model.

## First imported trace

Prerequisites are a running local CorpusLab API and web app. In **Settings → API keys**, select **Trace ingestion**, create a key, copy its one-time secret, and copy the current project ID shown on the same page.

```sh
export CORPUSLAB_API_URL=http://127.0.0.1:8080
export CORPUSLAB_API_KEY='the one-time clab_ secret'
export CORPUSLAB_PROJECT_ID='the project UUID shown in Settings'
./scripts/ingest-trace-example.sh
```

Open `http://127.0.0.1:5173/app/traces`. The `native-demo-001` trace is marked as a native import and shows its mapping status, permitted evidence, configuration, limitations, and span hierarchy. Repeating the command updates the same trace instead of creating a duplicate.

## Native JSON contract

Send `POST /api/v1/traces/ingest` with `Content-Type: application/json`. A browser session is accepted for local interactive use. External callers should send `Authorization: Bearer $CORPUSLAB_API_KEY` using a key scoped to `trace_ingest`. If an Authorization header is present but invalid, CorpusLab never falls back to a session cookie.

Schema version `1` requires `project_id`, `external_trace_id`, and `privacy_mode`. It accepts bounded query/prompt/answer fields, retrieval configuration, model labels, evidence, spans, known failure labels, evaluation state, timestamps, latency, and status. Unknown fields are rejected. The response reports the internal trace ID, `created`, `updated`, or `unchanged`, mapping status, accepted span count, and stable limitation codes.

Stored status is derived deterministically. A failed span, failed evaluation, or explicit failed status makes the trace failed. Warning spans, failure labels, partial mapping, or an explicit warning make it at least warning. Completed is used only when no failure or warning signal exists, and retries cannot downgrade an existing aggregate status.

## Direct OTLP/HTTP export

CorpusLab v1 accepts protobuf only at `POST /api/v1/otel/v1/traces`:

```sh
export OTEL_EXPORTER_OTLP_TRACES_ENDPOINT="$CORPUSLAB_API_URL/api/v1/otel/v1/traces"
export OTEL_EXPORTER_OTLP_TRACES_PROTOCOL=http/protobuf
export OTEL_EXPORTER_OTLP_TRACES_HEADERS="Authorization=Bearer%20$CORPUSLAB_API_KEY,x-corpuslab-project-id=$CORPUSLAB_PROJECT_ID"
export OTEL_EXPORTER_OTLP_COMPRESSION=none
```

`application/json`, gzip, OTLP/gRPC, and unauthenticated export are intentionally unsupported. OTLP content is always stored as `metadata_only`; telemetry attributes named like workspace, project, or privacy settings never grant authority or increase retention.

## Standard SDK and Collector example

The example uses OpenTelemetry Python 1.41.1 and Collector 0.158.0. It creates retrieval and generation spans locally and does not call a model.

```sh
python3 -m venv .venv
. .venv/bin/activate
pip install -r examples/trace-ingestion/python/requirements.txt
python examples/trace-ingestion/python/basic_ingest.py
```

For a Collector hop, run the standard Collector with `examples/trace-ingestion/otel-collector-config.yaml`, leave the same three `CORPUSLAB_*` variables set, and point the SDK exporter at `http://127.0.0.1:4318/v1/traces`. The Collector forwards protobuf with authentication and project headers. The result appears as an `otlp/http` import with a metadata-only limitation.

## Semantic mapping

Mapping precedence is `corpuslab.*`, OpenInference, then OpenTelemetry GenAI. Supported operations are retrieval, embedding, reranking, generation/chat, tool, eval/guardrail, and a retained `other` operation. Supported labels include provider/model/token/error metadata plus:

- `corpuslab.operation`, `corpuslab.retrieval_mode`, and `corpuslab.top_k`
- `corpuslab.provider`, `corpuslab.generation_model`, `corpuslab.embedding_model`, `corpuslab.ranker`, `corpuslab.configuration_label`
- `corpuslab.evidence.external_chunk_id`, `.rank`, `.score`, `.lexical_score`, `.semantic_score`, `.citation`, `.answer_support_status`, and `.answer_support_reason`
- string-array `corpuslab.failure_labels`, boolean `corpuslab.evaluation_passed`, and `corpuslab.evaluation_label`
- `corpuslab.query`, `.prompt`, `.answer`, `corpuslab.evidence.document_label`, and `.snippet` are recognized only so they can be stripped and reported as privacy limitations
- resource `service.name`, `service.version`, and `deployment.environment.name`, plus instrumentation scope name/version

Known content attributes—including query, prompt, completion, messages, document content, evidence snippets, exception messages, and stack traces—are discarded from OTLP imports. Unknown attributes are not persisted. OTLP span kind is retained as `internal`, `server`, `client`, `producer`, `consumer`, or `unspecified`. Because v1 OTLP is metadata-only, its original span name is treated as potentially content-bearing: Trace Debugger displays the canonical operation label instead and records `span_names_not_retained`. Unsupported operations remain visible as bounded `other` spans, and missing evidence produces `partially_mapped` rather than a false complete diagnosis.

## Privacy and downstream use

| Import mode        | Stored                                                                                  | Eval Lab                                                                | Reports                           |
| ------------------ | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | --------------------------------- |
| `metadata_only`    | IDs, structure, timing, status, ranks, scores, citations, and safe configuration labels | Disabled because no reproducible query is retained                      | Metadata only                     |
| `snippets_allowed` | The same metadata plus explicitly supplied bounded evidence snippets                    | Disabled because raw query/answer text is not an approved snippet       | Metadata or evidence snippets     |
| `full_local_only`  | Explicit bounded local query, prompt, answer, labels, span names, and evidence snippets | Allowed locally after selecting workspace-authorized CorpusLab evidence | Report creation and export denied |

Native metadata/snippet imports replace submitted span names with the canonical operation label and record `span_names_not_retained`; full-local imports retain the validated, trimmed original name. Control characters and names longer than 256 characters are rejected. React renders names as text, never trusted HTML, and imported span names are not included in report Markdown. Native span kind is optional for backward compatibility and defaults to `unspecified`.

Metadata and snippet imports cannot become Eval Lab cases because they retain no reproducible query. A full-local native import can become a local Eval case after the user selects workspace-authorized CorpusLab evidence. The API resolves the source trace inside the authenticated workspace, requires an exact query match, and derives immutable source, trace, and `full_local_only` provenance server-side. That provenance is stored with the case and copied into experiment snapshots/results. Local evaluation and comparison remain available, but CI evaluation and audit-report creation reject any affected dataset or experiment before persistence. Because no report can be created, Markdown, clipboard, download, and report API export paths receive none of that imported content.

The project policy is an upper bound: `LocalOnly` permits all native modes, `ExplicitSnippetSync` permits metadata/snippets, and `RedactedCloudSync` permits metadata only. CorpusLab rejects attempts to exceed it rather than silently downgrading. Privacy mode and schema version are immutable for an imported identity; use a new external trace ID to change either. The internal mapper version may advance on an incremental delivery; memory and PostgreSQL update the serialized trace and relational mapper-version column together.

## Limits and partial success

The request body limit is 1 MiB. Native requests allow 100 evidence records and 256 spans. OTLP allows 16 resource groups, 64 scope groups, 32 traces, 512 total spans, and 256 spans per trace. A span allows 64 attributes, 32 events, 32 links, and 32 attributes per event/link. Resource and scope limits are 32 and 16 attributes. Nested values stop at depth 8 and 32 entries. IDs are 128 characters; labels 256; document labels 512; raw attribute strings 4,096; query/prompt/answer 8,192; snippets 280. Top-k is 1–25, ranks 1–100, and scores must be finite within 0–1.

Request-wide authentication, project, content-type, encoding, decoding, or global-limit failures reject the export. After decoding, each OTLP trace is atomic: valid sibling traces persist while malformed traces contribute their exact span count to the standard `partial_success.rejected_spans` response. Its message contains only stable reason codes and counts.

## Stable rejection codes

| Area                        | Codes                                                                                                                                                                                                                                                               |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Authentication and project  | `unauthorized`, `insufficient_scope`, `project_header_required`, `invalid_project_id`, `project_not_found`                                                                                                                                                          |
| Transport and body          | `unsupported_content_type`, `unsupported_content_encoding`, `payload_limit_exceeded`, `malformed_json`, `malformed_protobuf`                                                                                                                                        |
| Native validation           | `invalid_native_trace`, `unsupported_schema_version`, `privacy_mode_not_permitted`, `invalid_identifier`, `invalid_label`, `invalid_content`, `invalid_number`, `collection_limit_exceeded`, `duplicate_evidence_id`, `duplicate_span_id`, `invalid_span_hierarchy` |
| Import identity and service | `import_identity_conflict`, `imported_trace_eval_not_permitted`, `trace_query_mismatch`, `imported_trace_query_immutable`, `full_local_eval_ci_not_permitted`, `full_local_eval_report_not_permitted`, `mapping_error`, `service_not_ready`, `storage_error`        |
| OTLP global limits          | `resource_span_limit_exceeded`, `scope_span_limit_exceeded`, `span_limit_exceeded`, `trace_limit_exceeded`, `resource_attribute_limit_exceeded`, `scope_attribute_limit_exceeded`                                                                                   |

Per-trace OTLP mapping failures are returned only as stable codes and counts in `partial_success.error_message`; they include invalid or conflicting IDs, timestamps, bounded numeric attributes, labels, nested attributes, hierarchy, and per-trace span limits. Error bodies and operational events never echo credentials, IDs, labels, or content.

## Troubleshooting

- `unauthorized` or `insufficient_scope`: create a non-revoked `trace_ingest` key and send the complete one-time secret.
- `project_not_found`: use the project ID displayed in the same workspace as the key.
- `unsupported_content_type`: native requires JSON; OTLP requires `application/x-protobuf`.
- `unsupported_content_encoding`: disable SDK or Collector compression.
- `*_limit_exceeded`: split the batch without increasing CorpusLab's safety limits.
- `import_identity_conflict`: schema and privacy mode cannot change for the same source/project/external trace ID.
- `partially_mapped`: inspect the listed limitations; add native bounded evidence when a full local diagnosis is needed.
