# Retrieval Playground

The Retrieval Playground searches persisted chunks, ranks local evidence, and returns an evidence summary with citations. It supports lexical, vector, and hybrid retrieval modes so chunk quality can be tested with both word overlap and local embeddings.

## API

`POST /api/v1/retrieval/query`

JSON request:

- `query`: required question text
- `top_k`: optional result count, defaults to `RAG_DEBUGGER_DEFAULT_TOP_K`, capped by `RAG_DEBUGGER_MAX_TOP_K`
- `retrieval_mode`: optional `lexical`, `vector`, or `hybrid`; defaults to `RAG_DEBUGGER_DEFAULT_RETRIEVAL_MODE`
- `source_ids`: optional source filter
- `document_ids`: optional document filter

The default search scope is all indexed documents.

Response:

- `run`: query, `top_k`, retrieval mode, latency, creation time, and run id
- `answer`: evidence summary text, status, and citations
- `hits`: ranked chunks with score, normalized score breakdown, matched terms, quality flags, evidence strength, duplicate count, snippet, source, document, and chunk metadata
- `embedding_status`: whether embeddings were required, ready, partial, missing, or stale for the query scope

`GET /api/v1/embeddings/status`

Returns the default local embedding model plus total, indexed, missing, stale, and last-indexed chunk counts.

`POST /api/v1/embeddings/index`

Synchronously indexes all chunks by default. The JSON body can optionally include `source_ids` or `document_ids` to restrict indexing.

`GET /api/v1/retrieval/evals`

Returns saved retrieval eval cases.

`POST /api/v1/retrieval/evals`

Creates an eval case with a query, top-k, and expected chunk/document ids.

`POST /api/v1/retrieval/evals/run`

Runs saved eval cases and records recall@k, precision@k, top matching rank, pass/fail, and latency.

## Retrieval Strategy

V1 retrieval is local-only and does not call hosted models or an LLM.

Modes:

- `lexical`: normalized term overlap, exact phrases, section-title matches, document path matches, and small metadata boosts.
- `vector`: local chunk embeddings and cosine similarity only.
- `hybrid`: vector similarity plus lexical, phrase, section, path, and metadata signals.

The default embedding provider is `local-hash-v1`, a deterministic CPU-local vectorizer. It is intentionally dependency-light and privacy-preserving. It is good enough to build the indexing, score explanation, and eval workflow, but it is not a transformer embedding model. The provider boundary is designed so a stronger ONNX/GPU embedding backend can replace it later without changing the API.

If `hybrid` or `vector` retrieval is requested before embeddings are indexed, the response returns `insufficient_evidence` with `embedding_status.readiness = "missing"` rather than silently falling back to lexical scoring.

## Extractive Answer

The evidence summary is assembled from the best retrieved snippets. Each snippet keeps a citation label such as `[1]` that points back to a document path, chunk ordinal, section title, and checksum prefix.

If no chunk provides enough evidence, the answer status is `insufficient_evidence` and the UI shows that no local evidence was found. The system should not invent an answer.

Heading-only chunks, duplicates, section-only matches, and chunks with weak evidence hints are not promoted as strong evidence. The UI shows `evidence_strength`, `quality_flags`, normalized score bars, and duplicate counts so teams can diagnose why a result ranked.

## Save As Trace

After a query completes, the Retrieval page can save the run as a Trace Debugger record. `Save trace` calls `POST /api/v1/traces/from-retrieval-run` with the retrieval run id, then `/app/traces` shows the run timeline, failure labels, ranked evidence, and rerun comparison controls.

## Persistence

Postgres stores local embeddings, retrieval playground runs, hits, and eval runs:

- chunk id, model provider/name, dimension, vector, checksum, and indexed timestamp
- query, top-k, answer text, answer status, latency, and created time
- retrieval mode
- full retrieval response JSON for trace creation
- hit rank, semantic/lexical/phrase/section/path/metadata score breakdown, chunk id, matched terms, snippet, and citation label
- trace debugger timelines and rerun experiments
- eval cases with expected chunks/documents
- eval run metrics and per-case results

## Known V1 Limits

- `local-hash-v1` is deterministic and local, but weaker than production transformer embeddings.
- No reranking model yet.
- No generated answer beyond extractive snippets.
- Eval pass/fail currently means at least one expected chunk/document appeared in the top-k results.
- Trace filters are not yet stored on the retrieval run, so query-input spans show filter counts only when the trace builder receives them in a later contract revision.
