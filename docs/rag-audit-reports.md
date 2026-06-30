# RAG Audit Reports

RAG Audit Reports turn retrieval diagnostics into a reviewable engineering deliverable. A report freezes the evidence, failure signals, configuration context, and recommended fixes from a Trace Debugger run, Eval Lab experiment, CI eval run, or manual investigation.

The current builder layer generates deterministic reports from saved traces, Eval Lab experiments, and CI eval runs. Persistence, APIs, UI integration, and Markdown rendering are delivered in separate reviewed tickets.

## Workflow

The complete workflow is:

1. Select a failed or suspicious trace, Eval Lab experiment, CI eval run, or manual investigation.
2. Generate a deterministic report from the stored diagnostic data.
3. Review findings, evidence references, failure labels, configuration context, and recommendations.
4. Choose a report privacy mode before sharing or exporting.
5. Export a professional Markdown report after the report has passed privacy review.

The report should answer:

- which query, dataset, experiment, or CI gate failed;
- which evidence was retrieved, expected, missing, duplicated, weak, or suspicious;
- which deterministic trace or Eval Lab failure labels were detected;
- what changed across trace reruns or experiment modes;
- whether chunking, embeddings, `top_k`, retrieval mode, reranking, metadata filters, citations, or corpus coverage should change; and
- which content can be shared without exposing raw customer documents.

## Domain Model

`crates/core/src/report.rs` defines the additive audit-report contracts.

`DebugReport` contains workspace and project ownership, a title, privacy-filtered subject, source, privacy mode, executive summary, deterministic context metadata, findings, recommendations, evidence references, and creation time.

Report sources are tagged by `type`:

- `trace`: a saved Trace Debugger run;
- `eval_experiment`: a saved Eval Lab experiment;
- `ci_eval_run`: a CI-triggered Eval Lab run;
- `manual`: a consultant or developer-authored investigation.

Findings have stable codes, severity, human-readable diagnosis, failure-label codes, and references to evidence labels. Recommendations have stable codes, priority, a typed remediation area, rationale, action, and related finding codes.

Evidence references can represent retrieved, expected, or missing evidence. Identifiers, ranking, path, section, checksum prefix, citation label, snippet, evidence strength, and quality flags are optional because their inclusion depends on the source and privacy mode.

## Privacy Modes

`DebugReportPrivacyMode` is separate from project-level `PrivacyMode`. Project privacy controls where project data may be processed; report privacy controls what one frozen report may contain or expose.

### `metadata_only`

Intended for the safest sharing boundary. Reports may include opaque IDs, retrieval modes, counts, latency, aggregate metrics, failure labels, gate status, and recommendations.

They must exclude raw query text, document paths, section titles, snippets, prompts, answers, and citation text. A subject should use a non-sensitive run or dataset descriptor.

### `snippets_allowed`

Allows explicitly approved query text, document labels, section titles, and bounded evidence snippets. Before export, the user must be able to preview the exact content leaving the local boundary.

This mode never permits full documents, uploaded binaries, embedding vectors, credentials, headers, or cookies.

### `full_local_only`

Allows complete local diagnostic detail for investigation. It is not shareable or exportable. A future export flow must reject this mode or require an explicit conversion to `metadata_only` or `snippets_allowed` followed by a redaction preview.

## Determinism

Reports do not use an LLM. Given the same stored source, privacy mode, and report-builder version, findings, evidence ordering, failure-label ordering, and recommendations must be stable. Generated IDs and timestamps are excluded from deterministic equality.

Configuration context uses an ordered map so Markdown and API output remain stable. Report builders must derive recommendations from explicit failure labels and metrics rather than open-ended text generation.

`crates/rag/src/reports` separates source builders, privacy filtering, and recommendation mapping. Callers supply `DebugReportBuildContext` with fixed ownership, ID, privacy mode, and timestamp so tests and later storage/API layers control non-deterministic values.

Trace builders require a saved retrieval response and include ranked evidence plus the latest rerun comparison. Eval builders include gate outcomes, mode metrics, expected/retrieved/missing evidence IDs, and failed cases. CI builders add branch, commit, config label, regression deltas, and newly failing case counts.

## Safe Sharing Rules

- Original uploaded binaries are never copied into a report.
- `metadata_only` is the default for shareable reports.
- Snippets require explicit selection or an explicit `snippets_allowed` action.
- Missing evidence is represented by identifiers and role, not invented text.
- Reports retain workspace and project ownership.
- Logs must never contain report subjects, snippets, query text, credentials, or serialized report bodies.
- Hosted sync and public report links are outside the MVP.

## Delivery Stack

The workflow is intentionally split into reviewable tickets:

1. Shared domain contracts and documentation.
2. Pure deterministic builders for traces and Eval Lab experiments.
3. Memory and Postgres persistence.
4. Authenticated `/api/v1/reports` routes and Markdown endpoint foundation.
5. Focused Reports list/detail UI.
6. Trace, Eval Lab, and CI integration actions.
7. Professional Markdown rendering and snapshot tests.

PDF export, billing, hosted sync, external LLM calls, and broad Reports-page redesign are not part of this stack.
