import { jsonRequest, requestJson } from "./client";
import type { EmbeddingModelInfo } from "./embeddings";
import type { RetrievalMode } from "./retrieval";

export interface CreateRetrievalEvalCaseRequest {
  name?: string;
  query: string;
  top_k?: number;
  expected_chunk_ids?: string[];
  expected_document_ids?: string[];
  notes?: string | null;
}

export interface RetrievalEvalCase {
  id: string;
  name: string;
  query: string;
  top_k: number;
  expected_chunk_ids: string[];
  expected_document_ids: string[];
  notes: string | null;
  created_at: string;
}

export interface RunRetrievalEvalRequest {
  case_ids?: string[];
  retrieval_mode?: RetrievalMode;
}

export interface RetrievalEvalResult {
  case_id: string;
  query: string;
  top_k: number;
  recall_at_k: number;
  precision_at_k: number;
  top_hit_rank: number | null;
  passed: boolean;
  expected_chunk_ids: string[];
  expected_document_ids: string[];
  retrieved_chunk_ids: string[];
  latency_ms: number;
}

export interface RetrievalEvalRun {
  id: string;
  retrieval_mode: RetrievalMode;
  case_count: number;
  passed_count: number;
  average_recall_at_k: number;
  average_precision_at_k: number;
  created_at: string;
  results: RetrievalEvalResult[];
}

export type RetrievalEvalGateStatus = "passed" | "failed";
export type RetrievalEvalFailureLabel =
  | "expected_evidence_missing"
  | "correct_document_wrong_chunk"
  | "low_precision"
  | "weak_evidence"
  | "missing_embeddings"
  | "heading_only_evidence"
  | "duplicate_evidence";
export type RetrievalEvalFailureSeverity = "warning" | "critical";

export interface RetrievalEvalDatasetSummary {
  id: string;
  name: string;
  description: string | null;
  case_count: number;
  latest_experiment_id: string | null;
  latest_gate: RetrievalEvalGate | null;
  latest_average_recall_at_k: number | null;
  latest_average_precision_at_k: number | null;
  updated_at: string;
}

export interface RetrievalEvalDataset {
  id: string;
  name: string;
  description: string | null;
  cases: RetrievalEvalCase[];
  created_at: string;
  updated_at: string;
}

export interface CreateRetrievalEvalDatasetRequest {
  name: string;
  description?: string | null;
}

export interface UpdateRetrievalEvalCaseRequest {
  name?: string;
  query?: string;
  top_k?: number;
  expected_chunk_ids?: string[];
  expected_document_ids?: string[];
  notes?: string | null;
}

export interface RunRetrievalEvalExperimentRequest {
  dataset_id: string;
  name?: string;
  modes?: RetrievalMode[];
  top_k?: number;
}

export interface RetrievalEvalExperiment {
  id: string;
  dataset_id: string;
  dataset_name: string;
  name: string;
  modes: RetrievalMode[];
  top_k: number;
  config_snapshot: RetrievalEvalConfigSnapshot;
  mode_results: RetrievalEvalModeResult[];
  comparison: RetrievalEvalComparison;
  gate: RetrievalEvalGate;
  failures: RetrievalEvalFailure[];
  created_at: string;
}

export interface RetrievalEvalConfigSnapshot {
  top_k: number;
  scoring_weights: Record<string, number>;
  embedding_model: EmbeddingModelInfo;
  dataset_case_count: number;
}

export interface RetrievalEvalModeResult {
  retrieval_mode: RetrievalMode;
  case_count: number;
  passed_count: number;
  average_recall_at_k: number;
  average_precision_at_k: number;
  mean_reciprocal_rank: number;
  citation_coverage: number;
  weak_evidence_count: number;
  missing_embedding_failures: number;
  latency_p50_ms: number;
  latency_p95_ms: number;
  case_results: RetrievalEvalCaseEvaluation[];
}

export interface RetrievalEvalCaseEvaluation {
  case_id: string;
  query: string;
  top_k: number;
  recall_at_k: number;
  precision_at_k: number;
  mrr: number;
  top_hit_rank: number | null;
  citation_coverage: number;
  weak_evidence_count: number;
  missing_embedding_failures: number;
  passed: boolean;
  expected_chunk_ids: string[];
  expected_document_ids: string[];
  retrieved_chunk_ids: string[];
  latency_ms: number;
  failures: RetrievalEvalFailure[];
}

export interface RetrievalEvalComparison {
  best_mode: RetrievalMode | null;
  mode_count: number;
  recall_delta: number;
  precision_delta: number;
  latency_delta_ms: number;
  summary: string;
}

export interface RetrievalEvalGate {
  status: RetrievalEvalGateStatus;
  average_recall_at_k: number;
  weak_evidence_rate: number;
  critical_failure_count: number;
  recall_threshold: number;
  weak_evidence_limit: number;
  reasons: string[];
}

export interface RetrievalEvalFailure {
  case_id: string;
  query: string;
  retrieval_mode: RetrievalMode;
  label: RetrievalEvalFailureLabel;
  severity: RetrievalEvalFailureSeverity;
  message: string;
  top_hit_rank: number | null;
}

export function listEvalLabDatasets(
  signal?: AbortSignal,
): Promise<RetrievalEvalDatasetSummary[]> {
  return requestJson<RetrievalEvalDatasetSummary[]>(
    "/api/v1/eval-lab/datasets",
    { signal },
  );
}

export function createEvalLabDataset(
  request: CreateRetrievalEvalDatasetRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalDataset> {
  return requestJson<RetrievalEvalDataset>(
    "/api/v1/eval-lab/datasets",
    jsonRequest("POST", request, signal),
  );
}

export function getEvalLabDataset(
  datasetId: string,
  signal?: AbortSignal,
): Promise<RetrievalEvalDataset> {
  return requestJson<RetrievalEvalDataset>(
    `/api/v1/eval-lab/datasets/${datasetId}`,
    { signal },
  );
}

export function createEvalLabCase(
  datasetId: string,
  request: CreateRetrievalEvalCaseRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalCase> {
  return requestJson<RetrievalEvalCase>(
    `/api/v1/eval-lab/datasets/${datasetId}/cases`,
    jsonRequest("POST", request, signal),
  );
}

export function updateEvalLabCase(
  caseId: string,
  request: UpdateRetrievalEvalCaseRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalCase> {
  return requestJson<RetrievalEvalCase>(
    `/api/v1/eval-lab/cases/${caseId}`,
    jsonRequest("PATCH", request, signal),
  );
}

export async function deleteEvalLabCase(
  caseId: string,
  signal?: AbortSignal,
): Promise<void> {
  await requestJson<{ deleted: boolean }>(
    `/api/v1/eval-lab/cases/${caseId}`,
    jsonRequest("DELETE", {}, signal),
  );
}

export function runEvalLabExperiment(
  request: RunRetrievalEvalExperimentRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalExperiment> {
  return requestJson<RetrievalEvalExperiment>(
    "/api/v1/eval-lab/experiments",
    jsonRequest("POST", request, signal),
  );
}

export function listEvalLabExperiments(
  signal?: AbortSignal,
): Promise<RetrievalEvalExperiment[]> {
  return requestJson<RetrievalEvalExperiment[]>(
    "/api/v1/eval-lab/experiments",
    { signal },
  );
}

export function compareEvalLabExperiment(
  experimentId: string,
  modes: RetrievalMode[],
  signal?: AbortSignal,
): Promise<RetrievalEvalComparison> {
  return requestJson<RetrievalEvalComparison>(
    `/api/v1/eval-lab/experiments/${experimentId}/compare`,
    jsonRequest("POST", { modes }, signal),
  );
}
