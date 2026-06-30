# Guided Demo

The CorpusLab guided demo is an additive, authenticated workflow that takes a new workspace from a sample corpus to a shareable RAG audit report in about five minutes. It exercises the same ingestion, embedding, retrieval, trace, report, storage, and privacy boundaries used by user-owned corpora.

## Walkthrough

1. Open Home and choose **Load sample corpus**.
2. Choose **Index sample**. Only the sample source is indexed.
3. Run the recommended account-recovery question, or choose the retention or GPU-indexing example.
4. Inspect citations and choose **Debug this run**.
5. On the saved trace, choose **Create audit report** and confirm `metadata_only` privacy.
6. Open the report and copy or download its Markdown export.

The Home checklist derives all six states from persisted records. It does not keep a separate progress flag and does not mark a step complete merely because a button was clicked.

## API

- `GET /api/v1/demo` returns the active workspace's sample IDs, persisted progress, and suggested queries.
- `POST /api/v1/demo/load` creates or repairs the sample corpus. It returns `201 Created` when documents are added and `200 OK` when the version is already complete.

Both routes require the normal workbench session. Suggested-query URLs contain only a stable ID such as `?demo_query=account_recovery`; the question text is resolved from the authenticated status response.

## Fixtures And Versioning

The API compiles three public Markdown fixtures into its binary:

- `fixtures/corpora/support_kb/account-recovery.md`
- `fixtures/corpora/policy_docs/data-retention.md`
- `fixtures/corpora/technical_docs/gpu-indexing.md`

Loading creates a workspace-owned project named `CorpusLab Guided Demo` and a source marked `corpuslab-guided-demo-v1`. Project, source, document, and chunk IDs are derived from SHA-256 inputs that include the workspace and version. Repeated or concurrent requests therefore target the same records.

Fixture content changes require a new version marker. Existing versions remain reproducible and are never silently rewritten as a different corpus.

## Shared Ingestion Path

`apps/api/src/ingestion.rs` owns document preparation for both multipart uploads and demo fixtures. It performs extraction, profile detection, extraction-quality analysis, structured chunking, checksums, duplicate detection, and chunk-quality annotation. The demo uses structured chunking with a target of 128 tokens and 16 overlap tokens.

Only ordinary project, source, document, chunk, embedding, retrieval, trace, and report records are persisted. No uploaded or compiled fixture binary is stored separately.

## Progress Derivation

- **Sample corpus loaded:** all three expected documents exist under the versioned source.
- **Chunks created:** each expected document has at least one chunk.
- **Embeddings indexed:** every demo-source chunk is indexed for the active model and none are stale.
- **Retrieval query run:** the latest persisted run contains evidence from the demo source.
- **Trace saved:** the latest trace references a run containing demo-source evidence.
- **Audit report generated:** the workspace contains a report sourced from that trace.

`DemoRepository` provides deterministic upserts plus source-specific retrieval and trace lookup in both MemoryStore and PostgresStore. Broad legacy repository scoping is intentionally unchanged by this milestone.

## Privacy

Loading the demo never deletes, resets, or uploads existing workspace data. Extraction and indexing are local. The final report defaults to `metadata_only`, which excludes query text, paths, sections, and snippets from shareable output. `snippets_allowed` requires explicit selection. `full_local_only` reports cannot be copied or downloaded until redacted.

## Troubleshooting

- If Docker is unavailable, start Docker Desktop and rerun `just db-up`.
- If migrations fail, confirm `DATABASE_URL` and run `just db-migrate`.
- If port `8080` or `5173` is occupied, stop the old API/web process before restarting.
- If persisted schema state is stale, run migrations before changing data.
- `docker compose down -v` deletes the local Postgres volume and all local CorpusLab data. It is destructive and is not part of normal demo setup.
