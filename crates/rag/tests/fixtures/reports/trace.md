# RAG trace audit

CorpusLab RAG Audit Report

## Executive Summary

Duplicate evidence weakened the result and the rerun improved latency.

## Deterministic Diagnosis

| Diagnosis field | Value |
| --- | --- |
| Outcome | mixed |
| Primary issue | duplicate\_evidence |
| Severity | warning |

This report looks mixed. Primary issue: Duplicate evidence crowded the ranking

## Report Source and Privacy Classification

| Field | Value |
| --- | --- |
| Report ID | 00000000-0000-0000-0000-000000000101 |
| Source type | trace |
| Source reference | 00000000-0000-0000-0000-000000000104 |
| Privacy mode | snippets\_allowed |
| Created at | 2026-06-30T08:15:30Z |

## System and Configuration Snapshot

| Configuration | Value |
| --- | --- |
| embedding model | local-hash-v1 |
| embedding readiness | ready |
| hit count | 1 |
| latency ms | 12 |
| latest rerun latency delta ms | -4 |
| latest rerun mode | lexical |
| latest rerun overlap count | 1 |
| latest rerun score delta | 0.125 |
| retrieval mode | hybrid |
| top k | 5 |

## Failing Queries or Cases

**Report subject:** When is the GPU index published?

### 1. Duplicate evidence weakened the result

- **Finding code:** `duplicate_evidence`
- **Severity:** warning
- **Evidence references:** `E1`

Two equivalent chunks competed in the ranked evidence.

### 2. Latest rerun changed retrieval behavior

- **Finding code:** `rerun_comparison`
- **Severity:** info
- **Evidence references:** `E1`

Top score changed by \+0.125 and latency by -4 ms.

## Evidence Diagnosis

### E1 - retrieved

| Evidence field | Value |
| --- | --- |
| Role | retrieved |
| Rank | 1 |
| Source ID | 00000000-0000-0000-0000-000000000121 |
| Document ID | 00000000-0000-0000-0000-000000000122 |
| Chunk ID | 00000000-0000-0000-0000-000000000123 |
| Checksum | abc123def456 |
| Citation | \[1\] |
| Evidence strength | medium |
| Document path | technical/gpu.md |
| Section | Index publication |

**Quality signals:** duplicate, weak_evidence

**Approved snippet:** Index publication requires checksum validation.

## Failure Labels

- `duplicate_evidence`

## Rerun, Experiment, and Regression Changes

- **Latest rerun changed retrieval behavior:** Top score changed by \+0.125 and latency by -4 ms.

| Change signal | Value |
| --- | --- |
| latest rerun latency delta ms | -4 |
| latest rerun mode | lexical |
| latest rerun overlap count | 1 |
| latest rerun score delta | 0.125 |

## Prioritized Recommendations

### 1. Remove duplicate chunks before indexing

- **Priority:** high
- **Area:** chunking
- **Recommendation code:** `deduplicate_chunks`
- **Related findings:** `duplicate_evidence`

**Rationale:** Equivalent chunks can occupy multiple top-k positions.

**Recommended action:** Deduplicate normalized chunk text and re-index the affected source.

## Privacy and Sharing Note

This export is classified `snippets_allowed`. It may contain explicitly approved query or case text, document labels, section titles, and evidence snippets capped at 280 characters. Review every included snippet before sharing.

Original uploaded binaries, complete documents, embedding vectors, credentials, and session data are not included.
