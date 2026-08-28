# ADR 0009: Versioned Portable Golden Datasets

- Status: Accepted
- Date: 2026-08-27

## Context

Eval Lab datasets were bound to local database UUIDs and could not be reviewed as stable files, validated before transfer, or prepared for the existing CI workflow. A portable format must preserve case identity and expected evidence without exporting corpus bodies, weakening workspace isolation, or making imported local work overwrite silently. MemoryStore and PostgreSQL must expose the same behavior, including for existing datasets created before portable keys.

## Decision

Golden datasets use a strict typed JSON schema with mandatory `schema_version: 1`. The dataset key is the validated canonical lowercase slug of its name. Canonical serialization fixes field order, sorts cases by a persisted human-readable `case_key`, sorts evidence references by checksum tuple, pretty-prints, and ends with one newline. It omits dataset/case/evidence UUIDs, timestamps, paths, raw document/chunk text, snippets, and embeddings; optional provenance retains its source trace UUID. Existing case keys are backfilled deterministically from normalized names with ordered numeric suffixes; new keys use the first available name/query-derived suffix.

Portable document identity is its checksum. Portable chunk identity is parent document checksum, chunk checksum, and ordinal. Import accepts only one exact identity resolved inside the authenticated workspace. Missing, ambiguous, or foreign evidence remains unresolved; there is no path/text/UUID fallback. Full export includes Eval-owned queries, notes, references, and optional provenance after an explicit warning. Metadata-only export omits content fields. Full-local datasets cannot use full export, and CI import cannot accept full-local provenance.

The only import modes are `create_new`, `merge_by_case_key`, `replace_dataset`, and `validate_only`. Every apply requires a successful dry run and a token bound to the workspace, canonical file, mode, target ID, and target update timestamp. Replace also requires explicit confirmation. Planning uses the existing 250-case and evidence limits. MemoryStore applies one validated plan under its lock; PostgreSQL locks the target revision and applies the same plan in one transaction. Validation-only, invalid, missing-token, and stale-token requests write nothing.

Tags, severity, importance, hosted synchronization, raw corpus transfer, and GitHub integration are not added because the current Eval domain does not support or require them.

## Consequences

Golden dataset files are stable engineering assets and can move between workspaces whose corpus checksums match. Checksum metadata and Eval queries can still be sensitive, so export remains explicit and privacy-labeled. Importing against a changed target or ambiguous corpus requires a new dry run or file repair instead of a silent merge. The `case_key` migration adds one required unique field; local UUIDs and existing API routes remain otherwise compatible.
