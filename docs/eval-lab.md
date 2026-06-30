# Eval Lab

Eval Lab is CorpusLab's quality-control center for retrieval systems. It turns important questions into reusable datasets, runs them across retrieval modes, explains failures, and produces a release gate that tells a team whether the current corpus and retrieval configuration are safe enough to ship.

## Core Concepts

- **Dataset:** a named set of retrieval cases, usually grouped by product area, customer workflow, compliance topic, or release gate.
- **Case:** a query plus expected evidence. Expected evidence can be exact chunk IDs, broader document IDs, or both.
- **Experiment:** one run of a dataset across one or more retrieval modes with a frozen config snapshot.
- **Mode result:** metrics for one retrieval mode, such as `hybrid`, `vector`, or `lexical`.
- **Comparison:** the cross-mode summary that identifies the best mode and the recall, precision, and latency spread.
- **Gate:** deterministic pass/fail rules for release readiness.
- **Failure:** a per-case diagnosis label that explains what went wrong.

## API Flow

Eval Lab routes live under `/api/v1/eval-lab`.

- `GET /datasets`: list datasets with case counts and latest gate summaries.
- `POST /datasets`: create a dataset.
- `GET /datasets/:dataset_id`: load a dataset and its cases.
- `POST /datasets/:dataset_id/cases`: create a case inside a dataset.
- `PATCH /cases/:case_id`: update a case.
- `DELETE /cases/:case_id`: delete a case.
- `POST /experiments`: run a dataset across selected modes.
- `GET /experiments`: list recent experiments.
- `GET /experiments/:experiment_id`: load one experiment.
- `POST /experiments/:experiment_id/compare`: compare selected modes from a saved experiment.

The older `/api/v1/retrieval/evals` endpoints remain available for compatibility. New UI workflows save cases into Eval Lab datasets.

## Metrics

Eval Lab calculates deterministic retrieval metrics from retrieved hits and expected evidence.

- `recall@k`: expected chunks or documents found within the top `k` results divided by expected evidence count.
- `precision@k`: retrieved hits that match expected chunks or documents divided by returned hits.
- `mrr`: mean reciprocal rank. A result at rank 1 scores `1.0`; rank 2 scores `0.5`.
- `top_hit_rank`: first rank where expected evidence appears.
- `citation_coverage`: expected evidence represented in cited or retrieved evidence.
- `weak_evidence_count`: hits whose retrieval metadata marks evidence as weak.
- `missing_embedding_failures`: cases blocked by missing embeddings in vector or hybrid mode.
- `latency_p50_ms` and `latency_p95_ms`: mode-level latency summaries for the dataset run.

## Failure Labels

Failures are deterministic so they can be used in CI and release reviews.

- `expected_evidence_missing`: no expected chunk or document appeared in retrieved evidence.
- `correct_document_wrong_chunk`: the right document was found, but the expected chunk was not.
- `low_precision`: too many irrelevant hits were returned.
- `weak_evidence`: retrieved evidence was present but not strong enough.
- `missing_embeddings`: vector or hybrid retrieval could not use required embeddings.
- `heading_only_evidence`: a heading-only chunk was retrieved as evidence.
- `duplicate_evidence`: duplicate chunks dominated the result set.

## Gate Rules

The default release gate passes when:

- average `recall@k` is at least `0.80`;
- there are no critical missing-embedding failures;
- no more than 20% of cases are weak-evidence cases.

Failed gates store human-readable reasons. Mission Control surfaces failed gates as critical risks so a team knows what to fix next.

## UI Workflow

Quality starts at `/app/evals` and uses focused detail routes.

1. Create or select a dataset from the Quality overview.
2. Open `/app/evals/datasets/:datasetId` and add cases with an expected document and chunk, or add one explicitly from a saved run.
3. Choose retrieval modes: lexical, vector, hybrid.
4. Pick `top_k`.
5. Run an experiment.
6. Open `/app/evals/experiments/:experimentId`. Inspect the gate result and failed cases first, then expand detailed metrics.
7. Use failed cases to improve documents, chunking, indexing, or retrieval config.
8. Create a privacy-classified audit report from the experiment detail when the gate decision is ready for review.

The Trace Debugger saves evidence into Quality with a note pointing back to the run. The user must choose both the target dataset and expected evidence. This prevents accidental labels and turns observed behavior into deliberate regression coverage.

Experiment Detail uses the same Reports-owned creation action as Trace Debugger. Metadata-only is the default; snippets or unrestricted local diagnostics require an explicit privacy selection before the report is generated.

## Storage Model

Postgres stores:

- `retrieval_eval_datasets`
- `retrieval_eval_cases.dataset_id`
- `retrieval_eval_experiments`

Existing `retrieval_eval_cases` remain valid. A default dataset named `Default retrieval dataset` is created by migration and legacy cases are backfilled into it.

The in-memory repository mirrors the same behavior for tests and local no-Postgres sessions.

## Writing Good Eval Cases

Good cases should represent the questions that would embarrass or block a real product if retrieval failed.

- Prefer customer, support, policy, product, contract, and technical decision questions.
- Use expected chunks when the exact evidence matters.
- Use expected documents when any section in the right document is acceptable.
- Add notes explaining why the case matters.
- Keep datasets small but high signal at first, then grow them by workflow.

## Why This Matters

Chunking and retrieval improvements are easy to eyeball and hard to trust. Eval Lab gives CorpusLab a measurement layer: every change to extraction, chunking, embeddings, scoring, reranking, or GPU acceleration can be compared against the same datasets. That is what makes future speed work meaningful: acceleration only matters if quality holds or improves.
