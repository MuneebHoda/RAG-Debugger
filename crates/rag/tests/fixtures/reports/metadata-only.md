# RAG trace audit

CorpusLab RAG Audit Report

## Executive Summary

Duplicate evidence weakened the result and the rerun improved latency.

## Report Source and Privacy Classification

| Field | Value |
| --- | --- |
| Report ID | 00000000-0000-0000-0000-000000000101 |
| Source type | trace |
| Source reference | 00000000-0000-0000-0000-000000000104 |
| Privacy mode | metadata\_only |
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

Query and case content is omitted by the `metadata_only` privacy policy.

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

**Quality signals:** duplicate, weak_evidence

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

This export is classified `metadata_only`. Query text, document paths, section titles, and evidence snippets are omitted. Review identifiers and operational metadata before sharing outside the workspace.

Original uploaded binaries, complete documents, embedding vectors, credentials, and session data are not included.
