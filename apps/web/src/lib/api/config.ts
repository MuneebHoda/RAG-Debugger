import { requestJson } from "./client";
import type { EmbeddingModelInfo } from "./embeddings";
import type { RetrievalMode } from "./retrieval";
import type { ChunkingStrategy } from "./sources";

export type DeploymentMode = "local" | "hybrid" | "hosted";

export interface ProductConfig {
  product: {
    name: string;
    workspace_name: string;
    deployment_mode: DeploymentMode;
  };
  ingestion: {
    max_files_per_request: number;
    max_file_bytes: number;
    max_request_bytes: number;
    preview_chunk_limit: number;
    supported_extensions: string[];
  };
  chunking: {
    target_tokens: number;
    overlap_tokens: number;
    strategy: ChunkingStrategy;
  };
  retrieval: {
    default_top_k: number;
    max_top_k: number;
    default_mode: RetrievalMode;
    min_evidence_score: number;
    min_semantic_similarity: number;
    answer_citation_limit: number;
    weights: Record<string, number>;
  };
  embedding: {
    model: EmbeddingModelInfo;
    provider_kind: "local_hash";
  };
  ui: {
    api_base_url: string;
    show_local_badges: boolean;
  };
}

export function getProductConfig(signal?: AbortSignal): Promise<ProductConfig> {
  return requestJson<ProductConfig>("/api/v1/config", { signal });
}
