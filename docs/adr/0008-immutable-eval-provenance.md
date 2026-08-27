# ADR 0008: Immutable Eval Provenance and Strict Baselines

- Status: Accepted
- Date: 2026-08-25

## Context

Eval Lab experiments preserved scores and a small configuration snapshot, but could not prove that two runs used the same dataset revision, corpus, chunks, embedding index, or ranking policy. Treating a merely similar run as a baseline could present configuration changes as regressions. Existing experiments are stored as JSONB snapshots and must remain readable.

## Decision

Every new manual or CI experiment captures a versioned provenance value before evaluation. One repository snapshot operation supplies source/document metadata and the exact candidate/embedding vector used by every case/mode evaluation. MemoryStore clones both under one mutex; Postgres reads both under one read-only `REPEATABLE READ` transaction, avoiding broad writer locks while preventing mixed snapshots. Identity collections and JSON object keys are sorted before SHA-256 hashing. Identity covers the dataset revision, corpus/document checksums and path fingerprints, chunking, privacy-safe chunk text/section/quality identity, chunk set, embedding configuration/index, retrieval modes, `top_k`, scoring, filters, and runtime flags. Build and CI metadata is informational and does not affect the identity fingerprint.

The provenance value is an additive optional property in the existing experiment JSONB. PostgreSQL needs no migration or backfill; absent provenance remains readable as `legacy_unknown`. Experiment IDs are append-only in MemoryStore and PostgreSQL. Both adapters verify that provenance workspace/project identities belong to the write authority.

Automatic baselines require identical identity provenance. An explicit earlier same-dataset comparison may cross configurations, but is `partially_compatible` and carries warnings, reason codes, and changed fields. Legacy records are never silently compatible.

Provenance stores opaque IDs, checksums, counts, configuration, and hashes, but no raw query, path, chunk/document text, section, vector, credential, or provider payload. Metadata-only reports expose aggregate fingerprints and compatibility context, not per-document checksums.

## Consequences

Regression labels now have reproducible configuration meaning, while legacy history and existing API fields remain compatible. Stored provenance increases experiment JSON size in proportion to source and document counts; raw content and vectors are deliberately excluded. Arbitrary model-inference provenance, hosted synchronization, and raw-document snapshots remain outside this decision.
