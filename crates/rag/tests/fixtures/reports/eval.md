# RAG evaluation audit

CorpusLab RAG Audit Report

## Executive Summary

The evaluation gate failed with one missing-evidence case.

## Report Source and Privacy Classification

| Field | Value |
| --- | --- |
| Report ID | 00000000-0000-0000-0000-000000000201 |
| Source type | eval experiment |
| Source reference | 00000000-0000-0000-0000-000000000204 |
| Privacy mode | snippets\_allowed |
| Created at | 2026-06-30T08:15:30Z |

## System and Configuration Snapshot

| Configuration | Value |
| --- | --- |
| best retrieval mode | hybrid |
| dataset case count | 4 |
| embedding model | local-hash-v1 |
| gate status | failed |
| hybrid latency p95 ms | 18 |
| hybrid mrr | 0.500 |
| hybrid precision at k | 0.400 |
| hybrid recall at k | 0.750 |
| top k | 5 |

## Failing Queries or Cases

**Report subject:** Release quality dataset

### 1. Which policy applies to failed indexing?

- **Finding code:** `expected_evidence_missing:case-1:hybrid`
- **Severity:** critical
- **Evidence references:** `M1`

The expected policy chunk was not retrieved.

### 2. Retrieval modes produced different outcomes

- **Finding code:** `retrieval_mode_comparison`
- **Severity:** info

Hybrid led lexical by 25% recall.

## Evidence Diagnosis

### M1 - missing

| Evidence field | Value |
| --- | --- |
| Role | missing |
| Chunk ID | 00000000-0000-0000-0000-000000000221 |

## Failure Labels

- `expected_evidence_missing`

## Rerun, Experiment, and Regression Changes

- **Retrieval modes produced different outcomes:** Hybrid led lexical by 25% recall.

| Change signal | Value |
| --- | --- |
| best retrieval mode | hybrid |
| gate status | failed |

## Prioritized Recommendations

### 1. Add the missing policy evidence

- **Priority:** critical
- **Area:** corpus_coverage
- **Recommendation code:** `expand_corpus_coverage`
- **Related findings:** `expected_evidence_missing:case-1:hybrid`

**Rationale:** No indexed chunk satisfies the expected evidence reference.

**Recommended action:** Ingest the current policy source and rerun the dataset gate.

## Privacy and Sharing Note

This export is classified `snippets_allowed`. It may contain explicitly approved query or case text, document labels, section titles, and evidence snippets capped at 280 characters. Review every included snippet before sharing.

Original uploaded binaries, complete documents, embedding vectors, credentials, and session data are not included.
