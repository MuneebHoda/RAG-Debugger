import { requestJson } from "./client";

export interface ByteRange {
  start: number;
  end: number;
}

export type ChunkingStrategy = "structured" | "smart_sections" | "whitespace";
export type DocumentProfile =
  | "general"
  | "technical_docs"
  | "policy_or_legal"
  | "support_kb"
  | "research_paper"
  | "code_docs"
  | "resume";
export type ExtractionQuality = "high" | "medium" | "low" | "unknown";
export type ChunkQualityFlag =
  | "heading_only"
  | "too_short"
  | "too_long"
  | "duplicate"
  | "low_text_density"
  | "extraction_warning"
  | "good_evidence_candidate";
export type ChunkSplitReason =
  | "section_boundary"
  | "token_limit"
  | "document_end"
  | "fallback_whitespace";

export interface ChunkPreview {
  id: string;
  document_id: string;
  ordinal: number;
  text: string;
  token_count: number;
  byte_range: ByteRange;
  checksum: string;
  strategy: ChunkingStrategy;
  section_title: string | null;
  split_reason: ChunkSplitReason;
  quality_flags: ChunkQualityFlag[];
  is_duplicate: boolean;
  text_density: number;
  evidence_score_hint: number;
}

export interface DocumentWarning {
  code: string;
  message: string;
}

export interface DocumentRecord {
  id: string;
  source_id: string;
  path: string;
  mime_type: string | null;
  checksum: string;
  byte_size: number;
  profile: DocumentProfile;
  extraction_quality: ExtractionQuality;
  warnings: DocumentWarning[];
}

export interface DocumentSummary {
  document: DocumentRecord;
  chunk_count: number;
}

export interface SourceRecord {
  id: string;
  project_id: string;
  name: string;
  kind: unknown;
  sync_policy: unknown;
  chunking: {
    target_tokens: number;
    overlap_tokens: number;
    strategy: ChunkingStrategy;
  };
}

export interface SourceSummary {
  source: SourceRecord;
  document_count: number;
  chunk_count: number;
  documents: DocumentSummary[];
}

export interface IngestionTotals {
  files_received: number;
  documents_created: number;
  chunks_created: number;
  failed_files: number;
}

export interface IngestionRun {
  id: string;
  source_id: string;
  status: string;
  totals: IngestionTotals;
  started_at: string;
  completed_at: string | null;
}

export interface DocumentIngestResult {
  file_name: string;
  status: "success" | "failure";
  document: DocumentRecord | null;
  chunk_count: number;
  preview_chunks: ChunkPreview[];
  error_code: string | null;
  message: string | null;
}

export interface IngestFilesResponse {
  source: SourceRecord;
  ingestion_run: IngestionRun;
  documents: DocumentIngestResult[];
  totals: IngestionTotals;
}

export function listSources(signal?: AbortSignal): Promise<SourceSummary[]> {
  return requestJson<SourceSummary[]>("/api/v1/sources", { signal });
}

export function ingestFiles(
  files: File[],
  config: {
    targetTokens: number;
    overlapTokens: number;
    strategy: ChunkingStrategy;
  },
  signal?: AbortSignal,
): Promise<IngestFilesResponse> {
  const formData = new FormData();
  for (const file of files) {
    formData.append("files[]", file);
  }
  formData.append("target_tokens", String(config.targetTokens));
  formData.append("overlap_tokens", String(config.overlapTokens));
  formData.append("chunking_strategy", config.strategy);

  return requestJson<IngestFilesResponse>(
    "/api/v1/sources/files",
    {
      method: "POST",
      body: formData,
      signal,
    },
    [201, 422],
  );
}

export function listDocumentChunks(
  documentId: string,
  signal?: AbortSignal,
): Promise<ChunkPreview[]> {
  return requestJson<ChunkPreview[]>(`/api/v1/documents/${documentId}/chunks`, {
    signal,
  });
}
