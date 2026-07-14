# Eval Lab

Eval Lab is CorpusLab's quality-control center for retrieval systems. It turns important questions into reusable datasets, runs them across retrieval modes, explains failures, and produces a release gate that tells a team whether the current corpus and retrieval configuration are safe enough to ship.

## Core Concepts

- **Dataset:** a named set of retrieval cases, usually grouped by product area, customer workflow, compliance topic, or release gate.
- **Case:** a query plus expected evidence. Exact chunk IDs mean that specific chunk must be retrieved. Broader document IDs mean any suitable chunk from that document can satisfy the case. Cases can contain both, but each must be selected explicitly.
- **Experiment:** one run of a dataset across one or more retrieval modes with a frozen config snapshot.
- **Mode result:** metrics for one retrieval mode, such as `hybrid`, `vector`, or `lexical`.
- **Comparison:** the cross-mode summary that identifies the best mode and the recall, precision, and latency spread.
- **Regression comparison:** the current experiment compared with a prior compatible experiment for the same dataset, `top_k`, and retrieval-mode set.
- **Trend:** a compact time series of recent experiment outcomes for one dataset.
- **Gate:** deterministic pass/fail rules for release readiness.
- **Failure:** a per-case diagnosis label that explains what went wrong.

## API Flow

Eval Lab routes live under `/api/v1/eval-lab`.

- `GET /datasets`: list datasets with case counts and latest gate summaries.
- `POST /datasets`: create a dataset.
- `GET /datasets/:dataset_id`: load a dataset and its cases.
- `GET /datasets/:dataset_id/experiments`: list dataset-scoped experiment history as compact summaries.
- `GET /datasets/:dataset_id/trends?limit=10`: load recent quality trend points and latest regression comparison.
- `POST /evidence/query`: search or resolve workspace-readable documents and chunks for the expected-evidence picker.
- `POST /datasets/:dataset_id/cases`: create a case inside a dataset.
- `PATCH /cases/:case_id`: update a case.
- `DELETE /cases/:case_id`: delete a case.
- `POST /experiments`: run a dataset across selected modes.
- `GET /experiments`: list recent experiments.
- `GET /experiments/:experiment_id`: load one experiment.
- `GET /experiments/:experiment_id/regression?baseline_id=<uuid>`: compare an experiment with an explicit baseline or the previous compatible run.
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

## Regression History

Eval Lab v2 treats saved experiments as release-history snapshots. The system compares a current experiment with the latest earlier experiment that has the same dataset, `top_k`, and sorted retrieval-mode set. Users can also request an explicit baseline by ID.

Regression comparison tracks:

- gate movement, such as `passed` to `failed`;
- recall, precision, MRR, citation coverage, weak-evidence case rate, missing embeddings, and p95 latency deltas;
- newly failing cases and recovered cases;
- changed top evidence and changed failure labels.

Overall classification is deterministic:

- `regressed` when a passed gate becomes failed, a case newly fails, or a metric crosses the regression threshold;
- `improved` when a failed gate becomes passed, a case recovers, or metrics improve without new regressions;
- `unchanged` when changes stay within thresholds.

Trend summaries default to the latest 10 experiments and clamp requests to 50 points. Trend points are chronological for graphing and review, while experiment history lists newest first.

Experiment Detail exposes the baseline choice so the comparison can be reviewed and revisited. The selector marks each candidate as:

- `fully compatible`: earlier experiment with the same dataset, `top_k`, and sorted retrieval-mode set;
- `partially compatible`: earlier experiment from the same dataset with different `top_k` or modes, selectable with a warning;
- `incompatible`: the current experiment or a newer experiment, visible but disabled.

The selected baseline is stored in the local URL as `baseline_id`. If no explicit baseline is selected, the API keeps using the automatic previous compatible run. When no baseline exists, the UI shows `No comparable baseline` instead of implying that the experiment is meaningfully unchanged.

## UI Workflow

Quality starts at `/app/evals` and uses focused detail routes.

1. Create or select a dataset from the Quality overview.
2. Open `/app/evals/datasets/:datasetId` and add cases with the expected-evidence picker. Search by question text, path, section, chunk text, or compact IDs, then choose exact chunks or document-level expectations explicitly.
3. Choose retrieval modes: lexical, vector, hybrid.
4. Pick `top_k`.
5. Run an experiment.
6. Open `/app/evals/experiments/:experimentId`. Inspect the gate result, regression summary, newly failed or recovered cases, and detailed metrics.
7. Use failed cases to improve documents, chunking, indexing, or retrieval config.
8. Create a privacy-classified audit report from the experiment detail when the gate decision is ready for review.

The Trace Debugger saves evidence into Quality with a note pointing back to the run. The user must choose both the target dataset and expected evidence. This prevents accidental labels and turns observed behavior into deliberate regression coverage.

Retrieval and Trace Debugger use the same shared save-to-Quality workflow. The panel shows retrieved chunks, lets the user choose the dataset, warns about duplicate normalized questions, shows readable document/chunk names, and submits only authenticated evidence IDs. It never asks users to manually paste UUIDs, and it never broadens an exact chunk expectation into a whole-document expectation.

Existing cases with stale or deleted expected evidence remain readable. When a case is edited, newly submitted evidence IDs are validated against evidence visible in the active workspace. Stale IDs are shown with `stale/deleted expected evidence` labels so the user can remove or replace them intentionally.

Case and experiment views show evidence states with text labels, not color alone:

- `expected document` and `expected exact chunk` for saved cases that have no retrieval context;
- `expected and retrieved`
- `expected but missing`
- `retrieved but not expected`
- `expected document retrieved, wrong chunk`
- `duplicate evidence`
- `weak evidence`
- `stale/deleted expected evidence`
- `metadata unavailable` when a retrieved or expected identifier cannot be resolved safely

An empty hit list has meaning only when an experiment actually ran. Saved dataset cases therefore show neutral expectations and never imply that retrieval failed. Completed experiments with zero hits show resolved expectations as missing.

Experiment results persist retrieved chunk IDs. The workbench resolves the union of expected and retrieved IDs through the authenticated evidence lookup endpoint before deriving states. Parent document IDs, paths, section titles, and authorized previews come from real chunk metadata; the UI never invents document identifiers. A document-level expectation succeeds when any resolved child chunk was retrieved. An exact chunk becomes `wrong chunk` only when a different resolved chunk from that same document was retrieved.

If metadata lookup fails or a retrieved ID cannot be resolved, CorpusLab shows `metadata unavailable` rather than inferring retrieved, missing, or wrong-chunk status. Case-level failure labels such as `weak_evidence`, `duplicate_evidence`, or `correct_document_wrong_chunk` remain visible without fabricating per-chunk details.

Experiment Detail uses the same Reports-owned creation action as Trace Debugger. Metadata-only is the default; snippets or unrestricted local diagnostics require an explicit privacy selection before the report is generated.

When a comparable baseline exists, experiment-sourced audit reports include regression classification, baseline experiment ID, newly failed case counts, recovered case counts, and metric deltas. Metadata-only reports keep this to IDs, labels, and metrics.

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
- Use expected chunks when the exact evidence matters; selecting a chunk adds only that chunk.
- Use expected documents when any section in the right document is acceptable; selecting a document adds only that document.
- Add notes explaining why the case matters.
- Keep datasets small but high signal at first, then grow them by workflow.

## Why This Matters

Chunking and retrieval improvements are easy to eyeball and hard to trust. Eval Lab gives CorpusLab a measurement layer: every change to extraction, chunking, embeddings, scoring, reranking, or GPU acceleration can be compared against the same datasets. That is what makes future speed work meaningful: acceleration only matters if quality holds or improves.
