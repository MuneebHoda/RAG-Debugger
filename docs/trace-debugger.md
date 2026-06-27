# Trace Debugger

The Trace Debugger turns a retrieval query into an inspectable RAG run. It answers four practical questions:

- What did CorpusLab do for this query?
- Which evidence was retrieved and cited?
- Which failure labels explain weak or surprising results?
- What changes when the same query is rerun with another retrieval mode or `top_k`?

The implementation is deterministic and local in this pass. It does not call a hosted LLM.

## User Flow

1. Upload documents on `/app/sources`.
2. Index local embeddings from `/app/retrieval`.
3. Ask a question in the Retrieval page.
4. Click `Debug this run`. CorpusLab saves the trace and opens `/app/traces/:traceId`.
5. Read the Summary diagnosis and recommended next action.
6. Inspect ranked evidence and ordered spans in the Evidence and Timeline tabs.
7. Rerun the trace with `lexical`, `vector`, or `hybrid` mode and compare score delta, latency delta, ranking movement, and overlap.

## API Routes

`GET /api/v1/traces`

Returns recent trace summaries. The response is optimized for the trace list and includes query, retrieval mode, latency, evidence strength, failure labels, span count, rerun count, and creation time.

`GET /api/v1/traces/:trace_id`

Returns the full trace timeline with spans, saved retrieval response, ranked evidence, citations, and rerun comparisons.

`POST /api/v1/traces/from-retrieval-run`

Creates a saved trace from a retrieval playground run. Request body:

```json
{
  "run_id": "optional retrieval run uuid"
}
```

If `run_id` is omitted, the API saves the latest retrieval response that was persisted by `POST /api/v1/retrieval/query`.

`POST /api/v1/traces/:trace_id/rerun`

Reruns the trace query with changed retrieval settings. Request body:

```json
{
  "retrieval_mode": "lexical",
  "top_k": 3,
  "source_ids": [],
  "document_ids": []
}
```

The response includes the updated trace and the latest rerun comparison.

## Code Structure

- `crates/core/src/trace.rs`: shared trace contracts, spans, failure labels, rerun comparison, and API request/response shapes.
- `crates/rag/src/tracing.rs`: deterministic trace construction, failure-label assignment, timeline spans, and rerun comparison metrics.
- `crates/storage/src/repository.rs`: repository methods for retrieval lookup and trace persistence.
- `crates/storage/src/memory.rs`: in-memory trace storage for tests and no-Docker development.
- `crates/storage/src/postgres/traces.rs`: Postgres trace storage with summary columns plus full JSON timeline.
- `apps/api/src/http/traces.rs`: Axum handlers for trace list, detail, create-from-retrieval, and rerun.
- `apps/web/src/features/workbench/traces/RunsPage.tsx`: searchable run list.
- `apps/web/src/features/workbench/traces/TraceDetailPage.tsx`: focused debugger route and tabs.
- `apps/web/src/features/workbench/traces/TraceDetailPanels.tsx`: diagnosis, evidence, timeline, comparison, and explicit Quality-case workflows.
- `apps/web/src/features/workbench/retrieval/RetrievalPage.tsx`: `Debug this run` action after a retrieval query.

## Storage Model

The trace migration adds:

- `retrieval_playground_runs.response_json`: lossless retrieval response used for trace creation after browser refreshes.
- `debug_traces`: one saved debugger trace per inspected run.
- `trace_rerun_experiments`: persisted rerun comparison records attached to traces.

`debug_traces` stores searchable/listable fields such as query, retrieval mode, status, evidence strength, failure labels, span count, rerun count, latency, and timestamps. The full timeline is also stored as JSON so the debugger can evolve without forcing every nested span field into relational columns immediately.

## Trace Spans

Current spans are:

- `query_input`: query text, selected retrieval mode, `top_k`, and filter counts.
- `retrieval`: hit count, top score, embedding readiness, and latency.
- `evidence_summary`: extractive answer status, citation count, and strongest evidence level.
- `eval_check`: placeholder status showing whether an eval was linked.
- `generation`: reserved for future generation metadata when hosted or local model generation is added.

## Failure Labels

The trace builder assigns labels from the saved retrieval response:

- `missing_document`: no chunks were retrieved.
- `missing_embedding_index`: vector or hybrid retrieval needed embeddings that were not indexed.
- `bad_embedding`: embeddings were missing or partially indexed.
- `weak_evidence`: answer status was insufficient or evidence was weak.
- `bad_ranking`: weak evidence was ranked.
- `duplicate_evidence`: duplicate evidence affected the result.
- `heading_only_evidence`: a heading-only chunk appeared as evidence.
- `bad_chunking`: chunk quality signals suggest chunk boundaries need review.

These labels are deterministic. They are meant to guide debugging, not replace human review.

## Rerun Comparison

A rerun keeps the original query and changes retrieval settings. The comparison records:

- `score_delta`: latest top score minus original top score.
- `latency_delta_ms`: latest latency minus original latency.
- `overlap_count`: how many chunk IDs appear in both original and rerun hits.
- `changed_rank_count`: how many overlapping chunks moved rank.

This helps users see whether lexical, vector, or hybrid retrieval is improving evidence quality or merely reshuffling the same weak chunks.

## UI Behavior

`/app/traces` is a searchable run list. Selecting a run opens `/app/traces/:traceId`, where the debugger has four focused tabs:

- **Summary** explains what happened, likely causes, and the recommended next action.
- **Evidence** shows ranked chunks, citations, quality, and normalized score bars.
- **Timeline** shows ordered query, retrieval, evidence, eval, and generation spans.
- **Compare** reruns the same question with changed retrieval settings and shows score, latency, overlap, and rank movement.

Summary also exposes **Add to Quality**. It requires an explicit dataset and expected chunk selection; CorpusLab never silently treats the first hit or first dataset as correct.

## Privacy

Traces are local in this pass. A trace stores retrieval query text, extracted chunk evidence, citations, metadata, and diagnostics. Original uploaded binaries are not stored. Hosted sync should later honor `PrivacyMode` and sync only approved metadata, redacted traces, or explicit snippets.

## Next Steps

- Link traces to eval cases so the Eval Check span can show pass/fail status.
- Let Reports reference saved traces as evidence sources.
- Add comments and reviewer notes on traces for team workflows.
- Store trace filters directly in `RetrievalQueryRun` so query-input spans can show exact source/document filters.
- Add prompt and generation spans when generation is implemented.
- Add API key and local collector upload paths for production traces.
