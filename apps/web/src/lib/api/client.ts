const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:8080";

export interface HealthResponse {
  status: string;
}

export interface ByteRange {
  start: number;
  end: number;
}

export type ChunkingStrategy = "structured" | "smart_sections" | "whitespace";
export type RetrievalMode = "lexical" | "vector" | "hybrid";
export type DeploymentMode = "local" | "hybrid" | "hosted";
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
export type RetrievalQualityFlag =
  | "duplicate"
  | "heading_only"
  | "too_short"
  | "weak_evidence"
  | "semantic_match"
  | "exact_term_match"
  | "section_only_match";
export type EvidenceStrength = "strong" | "medium" | "weak";
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

export interface DocumentWarning {
  code: string;
  message: string;
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

export interface IngestionRun {
  id: string;
  source_id: string;
  status: string;
  totals: IngestionTotals;
  started_at: string;
  completed_at: string | null;
}

export interface IngestionTotals {
  files_received: number;
  documents_created: number;
  chunks_created: number;
  failed_files: number;
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

export type ExtractiveAnswerStatus = "answered" | "insufficient_evidence";

export interface RetrievalQueryRequest {
  query: string;
  top_k?: number;
  retrieval_mode?: RetrievalMode;
  source_ids?: string[];
  document_ids?: string[];
}

export interface RetrievalQueryRun {
  id: string;
  query: string;
  top_k: number;
  retrieval_mode: RetrievalMode;
  latency_ms: number;
  created_at: string;
}

export interface RetrievalMatchedTerm {
  term: string;
  count: number;
}

export interface RetrievalScoreBreakdown {
  semantic: number;
  lexical: number;
  phrase: number;
  section: number;
  path: number;
  metadata: number;
}

export interface RetrievalCitation {
  label: string;
  chunk_id: string;
  document_id: string;
  document_path: string;
  chunk_ordinal: number;
  section_title: string | null;
  checksum_prefix: string;
  snippet: string;
}

export interface ExtractiveAnswer {
  status: ExtractiveAnswerStatus;
  text: string;
  citations: RetrievalCitation[];
}

export interface RetrievalQueryHit {
  rank: number;
  score: number;
  chunk: ChunkPreview;
  document: DocumentRecord;
  source: SourceRecord;
  matched_terms: RetrievalMatchedTerm[];
  score_breakdown: RetrievalScoreBreakdown;
  normalized_score_breakdown: RetrievalScoreBreakdown;
  snippet: string;
  citation: RetrievalCitation;
  quality_flags: RetrievalQualityFlag[];
  evidence_strength: EvidenceStrength;
  duplicate_count: number;
}

export interface RetrievalQueryResponse {
  run: RetrievalQueryRun;
  answer: ExtractiveAnswer;
  hits: RetrievalQueryHit[];
  embedding_status: RetrievalEmbeddingStatus;
}

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

export type FailureLabel =
  | "missing_document"
  | "bad_chunking"
  | "bad_embedding"
  | "bad_ranking"
  | "bad_prompt"
  | "unsupported_question"
  | "hallucinated_answer"
  | "weak_evidence"
  | "missing_embedding_index"
  | "duplicate_evidence"
  | "heading_only_evidence";

export type TraceStatus = "completed" | "warning" | "failed";
export type TraceSpanKind =
  | "query_input"
  | "retrieval"
  | "evidence_summary"
  | "eval_check"
  | "generation";
export type TraceSpanStatus = "succeeded" | "warning" | "failed";

export interface TraceSummary {
  id: string;
  query: string;
  retrieval_mode: RetrievalMode;
  latency_ms: number;
  evidence_strength: EvidenceStrength;
  failure_labels: FailureLabel[];
  span_count: number;
  rerun_count: number;
  created_at: string;
}

export interface TraceSpan {
  id: string;
  kind: TraceSpanKind;
  title: string;
  description: string;
  started_at: string;
  completed_at: string | null;
  latency_ms: number;
  status: TraceSpanStatus;
  detail: TraceSpanDetail;
}

export type TraceSpanDetail =
  | {
      type: "query_input";
      top_k: number;
      retrieval_mode: RetrievalMode;
      source_filter_count: number;
      document_filter_count: number;
    }
  | {
      type: "retrieval";
      hit_count: number;
      top_score: number;
      embedding_readiness: RetrievalEmbeddingReadiness;
    }
  | {
      type: "evidence_summary";
      answer_status: ExtractiveAnswerStatus;
      citation_count: number;
      strongest_evidence: EvidenceStrength;
    }
  | {
      type: "eval_check";
      checked: boolean;
      passed: boolean | null;
      message: string;
    }
  | {
      type: "generation";
      model: string | null;
      prompt_version: string | null;
      input_tokens: number;
      output_tokens: number;
    };

export interface TraceRerunComparison {
  id: string;
  request: RetrievalQueryRequest;
  response: RetrievalQueryResponse;
  score_delta: number;
  latency_delta_ms: number;
  overlap_count: number;
  changed_rank_count: number;
  created_at: string;
}

export interface Trace {
  id: string;
  project_id: string;
  input: string;
  output: string | null;
  started_at: string;
  completed_at: string | null;
  failure_labels: FailureLabel[];
  source_run_id: string | null;
  summary: string;
  status: TraceStatus;
  evidence_strength: EvidenceStrength | null;
  spans: TraceSpan[];
  retrieval: RetrievalQueryResponse | null;
  reruns: TraceRerunComparison[];
}

export interface CreateTraceFromRetrievalRunRequest {
  run_id?: string | null;
}

export interface RerunTraceRequest {
  retrieval_mode?: RetrievalMode;
  top_k?: number;
  source_ids?: string[];
  document_ids?: string[];
}

export interface TraceRerunResponse {
  trace: Trace;
  comparison: TraceRerunComparison;
}

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

export type OverviewHealthStatus =
  | "ready"
  | "needs_indexing"
  | "needs_eval_coverage"
  | "needs_documents";
export type OverviewTone = "neutral" | "good" | "warning" | "critical";
export type OverviewStepStatus = "complete" | "warning" | "pending" | "blocked";
export type OverviewSeverity = "info" | "warning" | "critical";
export type OverviewActionPriority = "primary" | "secondary";
export type OverviewActivityKind = "source" | "document" | "trace" | "eval";

export interface OverviewAction {
  id: string;
  label: string;
  detail: string;
  route: string;
  priority: OverviewActionPriority;
}

export interface OverviewHealth {
  score: number;
  status: OverviewHealthStatus;
  summary: string;
  primary_action: OverviewAction | null;
}

export interface OverviewMetric {
  id: string;
  label: string;
  value: string;
  detail: string;
  tone: OverviewTone;
}

export interface OverviewPipelineStep {
  id: string;
  label: string;
  status: OverviewStepStatus;
  count: number;
  detail: string;
  route: string;
  action_label: string;
}

export interface OverviewIssue {
  id: string;
  severity: OverviewSeverity;
  title: string;
  detail: string;
  route: string;
  action_label: string;
}

export interface OverviewActivity {
  id: string;
  kind: OverviewActivityKind;
  label: string;
  detail: string;
  route: string;
  created_at: string | null;
}

export interface OverviewDocumentProfile {
  profile: DocumentProfile;
  count: number;
  percentage: number;
}

export interface OverviewEvalRunSummary {
  id: string;
  retrieval_mode: RetrievalMode;
  case_count: number;
  passed_count: number;
  pass_rate: number;
  average_recall_at_k: number;
  average_precision_at_k: number;
  created_at: string;
}

export interface OverviewEvalExperimentSummary {
  id: string;
  dataset_name: string;
  gate_status: RetrievalEvalGateStatus;
  best_mode: RetrievalMode | null;
  average_recall_at_k: number;
  average_precision_at_k: number;
  failure_count: number;
  created_at: string;
}

export interface OverviewResponse {
  generated_at: string;
  health: OverviewHealth;
  metrics: OverviewMetric[];
  pipeline: OverviewPipelineStep[];
  issues: OverviewIssue[];
  actions: OverviewAction[];
  recent_activity: OverviewActivity[];
  document_mix: OverviewDocumentProfile[];
  embedding_status: EmbeddingStatus;
  latest_eval_run: OverviewEvalRunSummary | null;
  latest_eval_experiment: OverviewEvalExperimentSummary | null;
}

export async function getHealth(signal?: AbortSignal): Promise<HealthResponse> {
  const response = await fetch(`${API_BASE_URL}/healthz`, { signal });

  if (!response.ok) {
    throw new Error(`Health check failed with ${response.status}`);
  }

  return response.json() as Promise<HealthResponse>;
}

export async function getProductConfig(
  signal?: AbortSignal,
): Promise<ProductConfig> {
  const response = await fetch(`${API_BASE_URL}/api/v1/config`, { signal });
  return readJsonResponse<ProductConfig>(response);
}

export async function getOverview(
  signal?: AbortSignal,
): Promise<OverviewResponse> {
  const response = await fetch(`${API_BASE_URL}/api/v1/overview`, { signal });
  return readJsonResponse<OverviewResponse>(response);
}

export async function listSources(
  signal?: AbortSignal,
): Promise<SourceSummary[]> {
  const response = await fetch(`${API_BASE_URL}/api/v1/sources`, { signal });
  return readJsonResponse<SourceSummary[]>(response);
}

export async function ingestFiles(
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

  const response = await fetch(`${API_BASE_URL}/api/v1/sources/files`, {
    method: "POST",
    body: formData,
    signal,
  });

  return readJsonResponse<IngestFilesResponse>(response, [201, 422]);
}

export async function listDocumentChunks(
  documentId: string,
  signal?: AbortSignal,
): Promise<ChunkPreview[]> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/documents/${documentId}/chunks`,
    { signal },
  );
  return readJsonResponse<ChunkPreview[]>(response);
}

export async function queryRetrieval(
  request: RetrievalQueryRequest,
  signal?: AbortSignal,
): Promise<RetrievalQueryResponse> {
  const response = await fetch(`${API_BASE_URL}/api/v1/retrieval/query`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
    signal,
  });

  return readJsonResponse<RetrievalQueryResponse>(response);
}

export async function getEmbeddingStatus(
  signal?: AbortSignal,
): Promise<EmbeddingStatus> {
  const response = await fetch(`${API_BASE_URL}/api/v1/embeddings/status`, {
    signal,
  });
  return readJsonResponse<EmbeddingStatus>(response);
}

export async function indexEmbeddings(
  signal?: AbortSignal,
): Promise<EmbeddingIndexResponse> {
  const response = await fetch(`${API_BASE_URL}/api/v1/embeddings/index`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({}),
    signal,
  });

  return readJsonResponse<EmbeddingIndexResponse>(response);
}

export async function listRetrievalEvalCases(
  signal?: AbortSignal,
): Promise<RetrievalEvalCase[]> {
  const response = await fetch(`${API_BASE_URL}/api/v1/retrieval/evals`, {
    signal,
  });
  return readJsonResponse<RetrievalEvalCase[]>(response);
}

export async function createRetrievalEvalCase(
  request: CreateRetrievalEvalCaseRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalCase> {
  const response = await fetch(`${API_BASE_URL}/api/v1/retrieval/evals`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
    signal,
  });

  return readJsonResponse<RetrievalEvalCase>(response);
}

export async function runRetrievalEvals(
  request: RunRetrievalEvalRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalRun> {
  const response = await fetch(`${API_BASE_URL}/api/v1/retrieval/evals/run`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
    signal,
  });

  return readJsonResponse<RetrievalEvalRun>(response);
}

export async function listEvalLabDatasets(
  signal?: AbortSignal,
): Promise<RetrievalEvalDatasetSummary[]> {
  const response = await fetch(`${API_BASE_URL}/api/v1/eval-lab/datasets`, {
    signal,
  });
  return readJsonResponse<RetrievalEvalDatasetSummary[]>(response);
}

export async function createEvalLabDataset(
  request: CreateRetrievalEvalDatasetRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalDataset> {
  const response = await fetch(`${API_BASE_URL}/api/v1/eval-lab/datasets`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
    signal,
  });
  return readJsonResponse<RetrievalEvalDataset>(response);
}

export async function getEvalLabDataset(
  datasetId: string,
  signal?: AbortSignal,
): Promise<RetrievalEvalDataset> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/eval-lab/datasets/${datasetId}`,
    { signal },
  );
  return readJsonResponse<RetrievalEvalDataset>(response);
}

export async function createEvalLabCase(
  datasetId: string,
  request: CreateRetrievalEvalCaseRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalCase> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/eval-lab/datasets/${datasetId}/cases`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
      signal,
    },
  );
  return readJsonResponse<RetrievalEvalCase>(response);
}

export async function updateEvalLabCase(
  caseId: string,
  request: UpdateRetrievalEvalCaseRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalCase> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/eval-lab/cases/${caseId}`,
    {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
      signal,
    },
  );
  return readJsonResponse<RetrievalEvalCase>(response);
}

export async function deleteEvalLabCase(
  caseId: string,
  signal?: AbortSignal,
): Promise<void> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/eval-lab/cases/${caseId}`,
    {
      method: "DELETE",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
      signal,
    },
  );
  await readJsonResponse<{ deleted: boolean }>(response);
}

export async function runEvalLabExperiment(
  request: RunRetrievalEvalExperimentRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalExperiment> {
  const response = await fetch(`${API_BASE_URL}/api/v1/eval-lab/experiments`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
    signal,
  });
  return readJsonResponse<RetrievalEvalExperiment>(response);
}

export async function listEvalLabExperiments(
  signal?: AbortSignal,
): Promise<RetrievalEvalExperiment[]> {
  const response = await fetch(`${API_BASE_URL}/api/v1/eval-lab/experiments`, {
    signal,
  });
  return readJsonResponse<RetrievalEvalExperiment[]>(response);
}

export async function compareEvalLabExperiment(
  experimentId: string,
  modes: RetrievalMode[],
  signal?: AbortSignal,
): Promise<RetrievalEvalComparison> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/eval-lab/experiments/${experimentId}/compare`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ modes }),
      signal,
    },
  );
  return readJsonResponse<RetrievalEvalComparison>(response);
}

export async function listTraces(
  signal?: AbortSignal,
): Promise<TraceSummary[]> {
  const response = await fetch(`${API_BASE_URL}/api/v1/traces`, { signal });
  return readJsonResponse<TraceSummary[]>(response);
}

export async function getTrace(
  traceId: string,
  signal?: AbortSignal,
): Promise<Trace> {
  const response = await fetch(`${API_BASE_URL}/api/v1/traces/${traceId}`, {
    signal,
  });
  return readJsonResponse<Trace>(response);
}

export async function createTraceFromRetrievalRun(
  request: CreateTraceFromRetrievalRunRequest,
  signal?: AbortSignal,
): Promise<Trace> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/traces/from-retrieval-run`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
      signal,
    },
  );

  return readJsonResponse<Trace>(response);
}

export async function rerunTrace(
  traceId: string,
  request: RerunTraceRequest,
  signal?: AbortSignal,
): Promise<TraceRerunResponse> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/traces/${traceId}/rerun`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
      signal,
    },
  );

  return readJsonResponse<TraceRerunResponse>(response);
}

async function readJsonResponse<T>(
  response: Response,
  okStatuses: number[] = [200],
): Promise<T> {
  if (!okStatuses.includes(response.status)) {
    const text = await response.text();
    throw new Error(text || `Request failed with ${response.status}`);
  }

  return response.json() as Promise<T>;
}
