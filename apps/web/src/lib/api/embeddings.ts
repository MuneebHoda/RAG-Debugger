import { jsonRequest, requestJson } from "./client";

export interface EmbeddingModelInfo {
  provider: string;
  model_name: string;
  dimension: number;
}

export type RetrievalEmbeddingReadiness =
  | "not_required"
  | "ready"
  | "partial"
  | "missing";

export interface RetrievalEmbeddingStatus {
  readiness: RetrievalEmbeddingReadiness;
  required: boolean;
  model: EmbeddingModelInfo;
  total_chunks: number;
  indexed_chunks: number;
  missing_chunks: number;
  stale_chunks: number;
}

export interface EmbeddingStatus {
  model: EmbeddingModelInfo;
  total_chunks: number;
  indexed_chunks: number;
  missing_chunks: number;
  stale_chunks: number;
  last_indexed_at: string | null;
}

export interface EmbeddingIndexResponse {
  status: EmbeddingStatus;
  indexed_chunks: number;
}

export interface EmbeddingIndexRequest {
  source_ids?: string[];
  document_ids?: string[];
}

export function getEmbeddingStatus(
  signal?: AbortSignal,
): Promise<EmbeddingStatus> {
  return requestJson<EmbeddingStatus>("/api/v1/embeddings/status", { signal });
}

export function indexEmbeddings(
  request: EmbeddingIndexRequest = {},
  signal?: AbortSignal,
): Promise<EmbeddingIndexResponse> {
  return requestJson<EmbeddingIndexResponse>(
    "/api/v1/embeddings/index",
    jsonRequest("POST", request, signal),
  );
}
