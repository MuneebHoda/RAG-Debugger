# File Ingestion

File ingestion is the first complete RAG Debugger workflow: upload files, extract readable text, chunk it, persist metadata/chunks, and inspect the chunk preview in the Sources page.

## Supported Inputs

- `.txt`
- `.md`
- `.markdown`
- `.html`
- `.htm`
- `.pdf` with embedded text

PDF OCR is not included in v1. Scanned-image PDFs will usually produce empty or poor extracted text.

## Limits

- `RAG_DEBUGGER_MAX_FILES_PER_REQUEST`, default `10`
- `RAG_DEBUGGER_MAX_FILE_BYTES`, default `20 MB`
- `RAG_DEBUGGER_MAX_REQUEST_BYTES`, default `50 MB`
- Synchronous processing only

## API

`POST /api/v1/sources/files`

Multipart fields:

- `files[]`: one or more supported files
- `target_tokens`: optional unsigned integer, defaults to `512`
- `overlap_tokens`: optional unsigned integer, defaults to `64`
- `chunking_strategy`: optional strategy, defaults to `structured`

Supported chunking strategies:

- `structured`: detects document headings, keeps bullet groups and paragraphs together, and splits at section boundaries before falling back to token windows for oversized blocks.
- `smart_sections`: legacy alias for `structured`.
- `whitespace`: deterministic whitespace token windows with overlap. This is useful for debugging and comparing against the simplest possible splitter.

Every returned chunk includes:

- `strategy`: the strategy that produced the chunk
- `section_title`: detected section title when available
- `split_reason`: `section_boundary`, `token_limit`, `document_end`, or `fallback_whitespace`
- `token_count`, `byte_range`, and `checksum`
- `quality_flags`, `is_duplicate`, `text_density`, and `evidence_score_hint`

Responses:

- `201 Created` when at least one document is ingested.
- `422 Unprocessable Entity` when files were received but none could be ingested.
- Both success and partial-success responses include per-file results.

`GET /api/v1/sources`

Returns persisted sources with document and chunk counts.

`GET /api/v1/documents/:document_id/chunks`

Returns chunks for one document ordered by `ordinal`.

## Persistence

Postgres stores:

- project metadata
- source metadata
- source chunking strategy and token settings
- ingestion run totals and status
- document metadata
- detected document profile, extraction quality, and warnings
- extracted chunk text
- token counts, byte ranges, checksums, section titles, strategies, split reasons, quality flags, duplicate status, text density, and evidence hints

Original uploaded binaries are not stored. They are read from the multipart request, extracted in memory, and discarded after chunk persistence.

The `20260623203000_smart_chunking.sql` migration adds chunking metadata to existing local databases. Older uploaded sources and chunks default to `whitespace` and `token_limit` until they are re-uploaded.

## Structured Chunking Behavior

`structured` is optimized for general document corpora. It recognizes headings in technical docs, policies, support knowledge bases, research papers, code docs, contracts, wikis, and resumes. It also treats short uppercase or title-like lines without terminal punctuation as headings.

Within a section, the chunker packs paragraph blocks and consecutive bullet groups up to `target_tokens`. When a block is too large to fit, it uses the whitespace chunker for that block and marks the chunk with `fallback_whitespace`. Overlap is applied only when a section must be split because of token limits, not when a new section starts.

Use `whitespace` when you want a baseline comparison, when a document has unusual formatting, or when you are debugging how overlap changes chunk windows.

## Document Intelligence

Each document gets a detected profile:

- `general`
- `technical_docs`
- `policy_or_legal`
- `support_kb`
- `research_paper`
- `code_docs`
- `resume`

Each chunk gets quality signals. Heading-only chunks, duplicates, low-density chunks, and too-short chunks stay visible in Sources, but retrieval can avoid promoting them as strong evidence.

## Local Workflow

```sh
docker compose up -d postgres
cargo run -p rag-debugger-api
cd apps/web && npm run dev
```

Open `http://127.0.0.1:5173/app/sources`, upload files, then select a document to inspect persisted chunks.

After chunks look reasonable, open `http://127.0.0.1:5173/app/retrieval`, click `Index` in the Embeddings panel, and run a hybrid query. The retrieval page can save cited results into Eval Lab so future chunking or retrieval changes can be measured.
