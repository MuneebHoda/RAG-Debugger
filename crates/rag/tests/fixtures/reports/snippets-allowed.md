# Audit \*special\* \[customer\]

CorpusLab RAG Audit Report

## Executive Summary

User-controlled &lt;content&gt; is escaped before Markdown export.

## Report Source and Privacy Classification

| Field | Value |
| --- | --- |
| Report ID | 00000000-0000-0000-0000-000000000401 |
| Source type | manual investigation |
| Source reference | Customer \[alpha\] &lt;script&gt; |
| Privacy mode | snippets\_allowed |
| Created at | 2026-06-30T08:15:30Z |

## System and Configuration Snapshot

| Configuration | Value |
| --- | --- |
| document path | docs/\[private\].md |
| top k | 5 |

## Failing Queries or Cases

**Report subject:** Why did \#retrieval return \[unsafe\]\(link\)?

No deterministic failure findings were recorded.

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

**Approved snippet:** &lt;script&gt;alert\('x'\)&lt;/script&gt; xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx...

## Failure Labels

No failure labels were recorded.

## Rerun, Experiment, and Regression Changes

No rerun, mode-comparison, or regression change was recorded.

## Prioritized Recommendations

No remediation recommendation was generated.

## Privacy and Sharing Note

This export is classified `snippets_allowed`. It may contain explicitly approved query or case text, document labels, section titles, and evidence snippets capped at 280 characters. Review every included snippet before sharing.

Original uploaded binaries, complete documents, embedding vectors, credentials, and session data are not included.
