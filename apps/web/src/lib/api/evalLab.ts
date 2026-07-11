import { jsonRequest, requestJson } from "./client";
import type { EmbeddingModelInfo } from "./embeddings";
import type { RetrievalMode } from "./retrieval";
import type {
  ChunkQualityFlag,
  DocumentProfile,
  DocumentWarning,
  ExtractionQuality,
} from "./sources";

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

export interface QueryEvalLabEvidenceRequest {
  query?: string | null;
  document_ids?: string[];
  chunk_ids?: string[];
  limit?: number;
  include_chunks?: boolean;
}

export interface EvalLabEvidenceDocument {
  id: string;
  source_id: string;
  source_name: string;
  path: string;
  profile: DocumentProfile;
  extraction_quality: ExtractionQuality;
  warnings: DocumentWarning[];
  chunk_count: number;
}

export interface EvalLabEvidenceChunk {
  id: string;
  document_id: string;
  source_id: string;
  source_name: string;
  document_path: string;
  ordinal: number;
  text: string;
  token_count: number;
  checksum: string;
  section_title: string | null;
  quality_flags: ChunkQualityFlag[];
  is_duplicate: boolean;
  text_density: number;
  evidence_score_hint: number;
}

export interface QueryEvalLabEvidenceResponse {
  documents: EvalLabEvidenceDocument[];
  chunks: EvalLabEvidenceChunk[];
  unresolved_document_ids: string[];
  unresolved_chunk_ids: string[];
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

export type RetrievalEvalRegressionClassification =
  | "improved"
  | "regressed"
  | "unchanged";

export type RetrievalEvalRegressionMetric =
  | "recall_at_k"
  | "precision_at_k"
  | "mean_reciprocal_rank"
  | "citation_coverage"
  | "weak_evidence_case_rate"
  | "missing_embedding_failures"
  | "latency_p95_ms";

export interface RetrievalEvalExperimentSummary {
  id: string;
  dataset_id: string;
  dataset_name: string;
  name: string;
  modes: RetrievalMode[];
  top_k: number;
  best_mode: RetrievalMode | null;
  gate_status: RetrievalEvalGateStatus;
  average_recall_at_k: number;
  average_precision_at_k: number;
  mean_reciprocal_rank: number;
  citation_coverage: number;
  weak_evidence_case_rate: number;
  missing_embedding_failures: number;
  latency_p50_ms: number;
  latency_p95_ms: number;
  failure_count: number;
  created_at: string;
}

export interface RetrievalEvalTrendSummary {
  dataset_id: string;
  experiment_count: number;
  window_limit: number;
  latest_experiment_id: string | null;
  latest_gate_status: RetrievalEvalGateStatus | null;
  points: RetrievalEvalTrendPoint[];
  latest_regression?: RetrievalEvalRegressionComparison | null;
}

export interface RetrievalEvalTrendPoint {
  experiment_id: string;
  name: string;
  best_mode: RetrievalMode | null;
  gate_status: RetrievalEvalGateStatus;
  average_recall_at_k: number;
  average_precision_at_k: number;
  mean_reciprocal_rank: number;
  citation_coverage: number;
  weak_evidence_case_rate: number;
  latency_p95_ms: number;
  failure_count: number;
  created_at: string;
}

export interface RetrievalEvalRegressionComparison {
  current_experiment_id: string;
  baseline_experiment_id: string | null;
  classification: RetrievalEvalRegressionClassification;
  current_gate_status: RetrievalEvalGateStatus;
  baseline_gate_status: RetrievalEvalGateStatus | null;
  metric_deltas: RetrievalEvalMetricDelta[];
  newly_failed_cases: RetrievalEvalCaseRegression[];
  recovered_cases: RetrievalEvalCaseRegression[];
  changed_top_evidence_cases: RetrievalEvalCaseRegression[];
  changed_failure_label_cases: RetrievalEvalCaseRegression[];
  summary: string;
}

export interface RetrievalEvalMetricDelta {
  metric: RetrievalEvalRegressionMetric;
  current: number;
  baseline: number | null;
  delta: number;
  classification: RetrievalEvalRegressionClassification;
}

export interface RetrievalEvalCaseRegression {
  case_id: string;
  retrieval_mode: RetrievalMode;
  query: string;
  classification: RetrievalEvalRegressionClassification;
  current_passed: boolean | null;
  baseline_passed: boolean | null;
  current_top_hit_rank: number | null;
  baseline_top_hit_rank: number | null;
  current_retrieved_chunk_ids: string[];
  baseline_retrieved_chunk_ids: string[];
  current_failure_labels: RetrievalEvalFailureLabel[];
  baseline_failure_labels: RetrievalEvalFailureLabel[];
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

export type CiEvalRunStatus = "passed" | "failed";

export interface CiEvalRegressionSummary {
  baseline_run_id: string;
  recall_delta: number;
  precision_delta: number;
  mrr_delta: number;
  latency_delta_ms: number;
  newly_failed_case_count: number;
  summary: string;
}

export interface CiEvalReport {
  title: string;
  summary: string;
  gate: RetrievalEvalGate;
  experiment: RetrievalEvalExperiment;
  failed_cases: RetrievalEvalFailure[];
}

export interface CiEvalRun {
  id: string;
  workspace_id: string;
  dataset_id: string;
  dataset_name: string;
  experiment_id: string;
  status: CiEvalRunStatus;
  gate_status: RetrievalEvalGateStatus;
  branch: string | null;
  commit_sha: string | null;
  base_ref: string | null;
  head_ref: string | null;
  config_label: string;
  regression: CiEvalRegressionSummary | null;
  report: CiEvalReport;
  created_at: string;
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

export function queryEvalLabEvidence(
  request: QueryEvalLabEvidenceRequest,
  signal?: AbortSignal,
): Promise<QueryEvalLabEvidenceResponse> {
  return requestJson<QueryEvalLabEvidenceResponse>(
    "/api/v1/eval-lab/evidence/query",
    jsonRequest("POST", request, signal),
  );
}

export function listEvalLabDatasetExperiments(
  datasetId: string,
  signal?: AbortSignal,
): Promise<RetrievalEvalExperimentSummary[]> {
  return requestJson<RetrievalEvalExperimentSummary[]>(
    `/api/v1/eval-lab/datasets/${datasetId}/experiments`,
    { signal },
  );
}

export function getEvalLabDatasetTrends(
  datasetId: string,
  limit = 10,
  signal?: AbortSignal,
): Promise<RetrievalEvalTrendSummary> {
  return requestJson<RetrievalEvalTrendSummary>(
    `/api/v1/eval-lab/datasets/${datasetId}/trends?limit=${limit}`,
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

export function getEvalLabExperiment(
  experimentId: string,
  signal?: AbortSignal,
): Promise<RetrievalEvalExperiment> {
  return requestJson<RetrievalEvalExperiment>(
    `/api/v1/eval-lab/experiments/${experimentId}`,
    { signal },
  );
}

export function getEvalLabExperimentRegression(
  experimentId: string,
  baselineId?: string,
  signal?: AbortSignal,
): Promise<RetrievalEvalRegressionComparison> {
  const params = baselineId
    ? `?baseline_id=${encodeURIComponent(baselineId)}`
    : "";
  return requestJson<RetrievalEvalRegressionComparison>(
    `/api/v1/eval-lab/experiments/${experimentId}/regression${params}`,
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

export function listCiEvalRuns(signal?: AbortSignal): Promise<CiEvalRun[]> {
  return requestJson<CiEvalRun[]>("/api/v1/eval-lab/ci/runs", { signal });
}

export function getCiEvalReport(
  runId: string,
  signal?: AbortSignal,
): Promise<{ run: CiEvalRun; report: CiEvalReport }> {
  return requestJson<{ run: CiEvalRun; report: CiEvalReport }>(
    `/api/v1/eval-lab/ci/runs/${runId}/report`,
    { signal },
  );
}
