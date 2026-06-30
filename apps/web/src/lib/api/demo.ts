import { requestJson } from "./client";

export type DemoQueryId =
  | "account_recovery"
  | "data_retention"
  | "gpu_indexing";

export interface DemoSuggestedQuery {
  id: DemoQueryId;
  question: string;
  description: string;
  recommended: boolean;
}

export interface DemoProgress {
  sample_corpus_loaded: boolean;
  chunks_created: boolean;
  embeddings_indexed: boolean;
  document_count: number;
  chunk_count: number;
  indexed_chunk_count: number;
  retrieval_run_id: string | null;
  trace_id: string | null;
  report_id: string | null;
}

export interface DemoStatus {
  version: string;
  project_id: string | null;
  source_id: string | null;
  progress: DemoProgress;
  suggested_queries: DemoSuggestedQuery[];
}

export interface DemoLoadResponse {
  created_documents: number;
  status: DemoStatus;
}

export function getDemoStatus(signal?: AbortSignal): Promise<DemoStatus> {
  return requestJson<DemoStatus>("/api/v1/demo", { signal });
}

export function loadDemo(): Promise<DemoLoadResponse> {
  return requestJson<DemoLoadResponse>(
    "/api/v1/demo/load",
    { method: "POST" },
    [200, 201],
  );
}
