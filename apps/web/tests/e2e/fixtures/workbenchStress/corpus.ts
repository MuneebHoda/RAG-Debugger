import type {
  ChunkPreview,
  DocumentRecord,
  SourceRecord,
  SourceSummary,
} from "../../../../src/lib/api/sources";

import { stressIds, stressValues, unbrokenToken } from "./shared";

export const stressSourceRecord = {
  id: stressIds.source,
  project_id: stressIds.project,
  name: `Enterprise corpus ${unbrokenToken}`,
  kind: { FileSet: { root_hint: "browser-upload" } },
  sync_policy: "Manual",
  chunking: {
    target_tokens: 512,
    overlap_tokens: 64,
    strategy: "structured",
  },
} satisfies SourceRecord;

export const stressDocumentRecord = {
  id: stressIds.document,
  source_id: stressIds.source,
  path: stressValues.documentPath,
  mime_type: "text/markdown",
  checksum: unbrokenToken,
  byte_size: 256_000,
  profile: "technical_docs",
  extraction_quality: "low",
  warnings: [
    {
      code: "dense_extraction_warning",
      message: `Extraction warning ${unbrokenToken}`,
    },
  ],
} satisfies DocumentRecord;

export const stressChunk = {
  id: stressIds.chunk,
  document_id: stressIds.document,
  ordinal: 27,
  text: stressValues.snippet,
  token_count: 512,
  byte_range: { start: 131_072, end: 132_096 },
  checksum: unbrokenToken,
  strategy: "structured",
  section_title: `Failure analysis ${unbrokenToken}`,
  split_reason: "token_limit",
  quality_flags: [
    "too_long",
    "duplicate",
    "low_text_density",
    "extraction_warning",
  ],
  is_duplicate: true,
  text_density: 0.12,
  evidence_score_hint: 0.18,
} satisfies ChunkPreview;

export const stressSource = {
  source: stressSourceRecord,
  document_count: 1,
  chunk_count: 28,
  documents: [{ document: stressDocumentRecord, chunk_count: 28 }],
} satisfies SourceSummary;
