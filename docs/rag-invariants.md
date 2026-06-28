# RAG Invariants

This document defines the behavior CorpusLab must preserve while retrieval, tracing, and evaluation implementations evolve. An invariant is an externally observable guarantee, not an implementation preference. Changes that intentionally alter an invariant require tests, documentation, and a changelog entry.

## Retrieval Runs

Every successful retrieval response must:

- include a unique run ID, original query, retrieval mode, bounded `top_k`, latency, and creation timestamp;
- rank hits in a stable order and identify each hit by source, document, chunk ID, ordinal, and checksum;
- expose raw and normalized score components instead of only a final score;
- identify matched terms, quality flags, evidence strength, and duplicate count;
- return explicit embedding readiness for vector and hybrid modes;
- return a deterministic evidence summary and citations derived from ranked evidence;
- suppress repeated chunks, checksums, and normalized text from independent evidence positions;
- avoid promoting heading-only or duplicate chunks as strong answer evidence; and
- report insufficient local evidence rather than manufacture an answer.

Lexical mode does not require embeddings. Vector and hybrid modes must report missing or partial embeddings explicitly; they must not silently present a lexical-only result as vector-ready.

Retrieval is deterministic for the same corpus state, query, filters, mode, `top_k`, embedding model, and scoring configuration. Timing and generated IDs are excluded from deterministic equality.

## Trace Debugger

A saved trace must preserve:

- the original query and source retrieval response;
- ordered query, retrieval, evidence-summary, and eval-check spans;
- ranked evidence, citations, latency, embedding readiness, and score metadata;
- deterministic failure labels derived from response metadata; and
- a status and plain-language summary consistent with those labels.

A rerun must preserve the original trace and store the changed request and new response as a comparison. The comparison currently guarantees top-score delta, latency delta, evidence overlap count, and changed-rank count. Added and removed evidence can be derived from the two stored responses; explicit added/removed fields require a versioned contract change.

Trace diagnosis uses a stable label order. A condition may imply more than one label: duplicate or heading-only evidence also indicates bad chunking, and missing indexes also indicate bad embeddings. Labels are deduplicated without reordering.

## Eval Lab

Every eval case must preserve its query, `top_k`, expected chunk IDs, expected document IDs, notes, and dataset membership. An experiment must preserve the dataset version, selected modes, retrieval configuration snapshot, embedding metadata, per-case results, aggregate metrics, failures, and gate outcome.

Metrics are deterministic from stored results:

- `recall@k`: distinct expected chunks and documents retrieved divided by all expected chunks and documents;
- `precision@k`: retrieved hits matching an expected chunk or document divided by all retrieved hits;
- `MRR`: reciprocal rank of the first matching hit;
- citation coverage: cited hits divided by retrieved hits, capped at one;
- latency p50/p95: nearest-rank values from sorted case latencies.

The default gate passes only when the best mode reaches at least `0.80` average recall, has no critical failures, and has no more than a `0.20` weak-evidence rate. Gate decisions and reasons must be stored with the experiment so later configuration changes cannot rewrite history.

## Failure Labels

Trace labels cover:

- missing document;
- missing embedding index or degraded embeddings;
- weak evidence or weak ranked hits;
- duplicate evidence;
- heading-only evidence; and
- chunking or ranking degradation implied by those signals.

Eval Lab adds expectation-aware labels:

- expected evidence missing;
- correct document but wrong chunk;
- low precision;
- weak evidence;
- missing embeddings;
- heading-only evidence; and
- duplicate evidence.

Failure-label tests assert the complete ordered label vector for representative degraded responses. New labels must be additive within `/api/v1` unless the compatibility note explicitly defines a breaking migration.

## Privacy And Local-First Behavior

- Original uploaded binaries are not persisted.
- Extracted text, chunks, embeddings, retrieval responses, traces, and eval results remain in the configured CorpusLab storage boundary.
- The local embedding and deterministic diagnosis paths make no hosted model calls.
- Fixture corpora contain synthetic product documentation only and must never contain customer or contributor data.
- Logs and diagnostics must prefer IDs, counts, statuses, and checksums over raw query or document text.

## Regression Fixtures

Public fixtures live under `fixtures/`:

- `corpora/support_kb` includes duplicate support text and nearby but different account workflows;
- `corpora/policy_docs` includes a current rule and a superseded contradictory rule;
- `corpora/technical_docs` includes cross-section GPU indexing evidence;
- `expected/*.json` records representative retrieval, trace, and Eval Lab expectations.

The fixtures are intentionally small, synthetic, reviewable, and versioned. Rust tests validate that every expected fixture is valid JSON and that every corpus profile remains present. Focused engine tests construct typed responses directly so contract drift fails at compile time.

## Change Checklist

When changing retrieval, trace, or eval behavior:

1. Identify affected invariants.
2. Add or update deterministic unit tests.
3. Update public fixtures when the scenario changes.
4. Preserve `/api/v1` compatibility or document the migration.
5. Update this document, the technical handbook, testing guide, and changelog.
6. Run `just rust-check`; run `just ci-check` for API or workbench behavior changes.
