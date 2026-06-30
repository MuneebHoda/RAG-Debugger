# RAG Audit Reports

RAG Audit Reports turn retrieval diagnostics into a reviewable engineering deliverable. A report freezes the evidence, failure signals, configuration context, and recommended fixes from a Trace Debugger run, Eval Lab experiment, CI eval run, or manual investigation.

The current workflow generates deterministic reports from saved traces, Eval Lab experiments, and CI eval runs. Reports are persisted through workspace-scoped memory and Postgres repositories, exposed through authenticated report APIs, reviewed in the Reports workbench, and exported through a professional privacy-aware Markdown template.

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

`DebugReport` contains workspace and project ownership, a title, privacy-filtered subject, source, privacy mode, executive summary, deterministic context metadata, findings, recommendations, evidence references, an optional structured diagnosis snapshot, and creation time.

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

Reports do not use an LLM. The same analyzer in `crates/rag/src/diagnosis.rs` supplies retrieval, trace, Eval Lab, and report diagnosis. Given the same stored source, privacy mode, debugger policy, and report-builder version, findings, evidence ordering, failure-label ordering, score explanations, and recommendations must be stable. Generated IDs and timestamps are excluded from deterministic equality.

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

## Persistence

`ReportRepository` stores complete `DebugReport` snapshots and requires a workspace ID for list and detail reads. Multiple snapshots may be created from the same trace or experiment. Postgres stores the canonical report JSON alongside workspace, project, source, privacy, title, subject, and timestamp columns used for ownership and indexing.

The default generation path is `metadata_only`; reports containing explicitly approved snippets or full local diagnostics remain inside the same configured storage boundary. No report persistence path sends data to an external service.

## API

Authenticated workbench routes provide report list/detail, source-specific creation, and Markdown export:

- `GET /api/v1/reports`
- `GET /api/v1/reports/:report_id`
- `POST /api/v1/reports/from-trace`
- `POST /api/v1/reports/from-experiment`
- `POST /api/v1/reports/from-ci-run`
- `GET /api/v1/reports/:report_id/export.md`

Creation defaults to `metadata_only` when `privacy_mode` is omitted and returns `201 Created`. List, detail, and export reads are scoped to the authenticated workspace. CI creation also verifies that the CI run belongs to the active workspace.

Markdown responses use `text/markdown; charset=utf-8` and a stable attachment filename. `full_local_only` export returns `422 Unprocessable Entity`; users must create a redacted metadata or snippet report instead.

## Markdown Export

The renderer in `crates/rag/src/reports/markdown.rs` produces these stable sections:

1. Executive Summary
2. Deterministic Diagnosis, when the report contains a v2 diagnosis snapshot
3. Report Source and Privacy Classification
4. System and Configuration Snapshot
5. Failing Queries or Cases
6. Evidence Diagnosis
7. Failure Labels
8. Rerun, Experiment, and Regression Changes
9. Prioritized Recommendations
10. Privacy and Sharing Note

Context metadata is emitted in ordered-map order. Findings, evidence, failure labels, and recommendations preserve contract order, with duplicate failure labels removed by first occurrence. User-controlled Markdown punctuation is escaped and angle brackets cannot become raw HTML.

`metadata_only` export performs defense-in-depth filtering: it omits subject content, document paths, section titles, evidence snippets, and context keys associated with queries, prompts, answers, text, paths, or sections. Structured diagnosis contains only IDs, scores, labels, and deterministic remediation text, so it remains safe under this mode. `snippets_allowed` may include approved content, but evidence snippets are capped at 280 characters both during report construction and again during export. `full_local_only` remains non-exportable.

Checked-in fixtures under `crates/rag/tests/fixtures/reports` lock exact trace, eval, CI, metadata-only, and snippets-allowed output. Maintainers can intentionally regenerate them with:

```sh
UPDATE_REPORT_FIXTURES=1 cargo test -p rag-debugger-rag --test report_markdown_snapshots
```

Fixture changes require review as public report-format changes, not routine test churn.

## Workbench

`/app/reports` leads with saved audit reports and a source-driven creation form. Users select a saved trace or Eval Lab experiment, choose an explicit privacy mode, and open the generated snapshot directly. Existing CI gate failures, weak traces, and corpus warnings remain visible as report candidates.

`/app/reports/:reportId` presents the executive summary, source and configuration context, findings, failure labels, evidence references, recommendations, and privacy classification. Shareable reports can copy the Markdown export through the authenticated API. The copy action is disabled for `full_local_only` reports and explains that redaction is required.

Frontend ownership lives in `apps/web/src/features/workbench/reports`; route files under `apps/web/src/pages` remain thin compatibility exports. The typed API boundary is `apps/web/src/lib/api/reports.ts`, and TanStack Query hooks own report loading, creation, caching, and mutation invalidation.

Trace Detail, Eval experiment detail, and failed CI gate rows expose the same Reports-owned `Create audit report` action. The action always opens a confirmation panel, defaults to `metadata_only`, and requires an explicit privacy selection before generation. Successful creation opens the new report directly. Submission is synchronously guarded so rapid repeated clicks cannot create duplicate snapshots.

## Delivery Stack

The workflow is intentionally split into reviewable tickets:

1. Shared domain contracts and documentation.
2. Pure deterministic builders for traces and Eval Lab experiments.
3. Memory and Postgres persistence.
4. Authenticated `/api/v1/reports` routes and Markdown endpoint foundation.
5. Focused Reports list/detail UI. **Implemented.**
6. Trace, Eval Lab, and CI integration actions. **Implemented.**
7. Professional Markdown rendering and snapshot tests. **Implemented.**

PDF export, billing, hosted sync, external LLM calls, and broad Reports-page redesign are not part of this stack.
