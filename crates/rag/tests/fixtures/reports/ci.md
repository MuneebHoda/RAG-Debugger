# RAG CI gate audit

CorpusLab RAG Audit Report

## Executive Summary

The CI gate failed after the indexing configuration changed.

## Deterministic Diagnosis

| Diagnosis field | Value |
| --- | --- |
| Outcome | failing |
| Primary issue | missing\_expected\_evidence |
| Severity | critical |

This report looks failing. Primary issue: Expected evidence was not retrieved

## Report Source and Privacy Classification

| Field | Value |
| --- | --- |
| Report ID | 00000000-0000-0000-0000-000000000301 |
| Source type | CI eval run |
| Source reference | 00000000-0000-0000-0000-000000000304 |
| Privacy mode | metadata\_only |
| Created at | 2026-06-30T08:15:30Z |

## System and Configuration Snapshot

| Configuration | Value |
| --- | --- |
| ci branch | feature/index-v2 |
| ci commit sha | abc123def456 |
| ci config label | gpu-index-v2 |
| ci gate status | failed |
| ci latency delta ms | 5 |
| ci mrr delta | -0.100 |
| ci newly failed case count | 1 |
| ci precision delta | -0.100 |
| ci recall delta | -0.200 |
| top k | 5 |

## Failing Queries or Cases

Query and case content is omitted by the `metadata_only` privacy policy.

### 1. CI regression comparison

- **Finding code:** `ci_regression`
- **Severity:** critical

One case failed after the indexing change.

## Evidence Diagnosis

No evidence references were available for this report.

## Failure Labels

- `expected_evidence_missing`

## Rerun, Experiment, and Regression Changes

- **CI regression comparison:** One case failed after the indexing change.

| Change signal | Value |
| --- | --- |
| ci gate status | failed |
| ci latency delta ms | 5 |
| ci mrr delta | -0.100 |
| ci newly failed case count | 1 |
| ci precision delta | -0.100 |
| ci recall delta | -0.200 |

## Prioritized Recommendations

### 1. Review newly failing CI cases

- **Priority:** critical
- **Area:** retrieval_mode
- **Recommendation code:** `review_ci_regression`
- **Related findings:** `ci_regression`

**Rationale:** The current configuration introduced a new retrieval failure.

**Recommended action:** Compare the baseline and head configuration before release.

## Privacy and Sharing Note

This export is classified `metadata_only`. Query text, document paths, section titles, and evidence snippets are omitted. Review identifiers and operational metadata before sharing outside the workspace.

Original uploaded binaries, complete documents, embedding vectors, credentials, and session data are not included.
