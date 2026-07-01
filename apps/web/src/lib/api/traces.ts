import { jsonRequest, requestJson } from "./client";
import type { RetrievalEmbeddingReadiness } from "./embeddings";
import type {
  DiagnosisFailureCode,
  DiagnosisOutcome,
  EvidenceStrength,
  EvidenceDiagnosisSummary,
  ExtractiveAnswerStatus,
  RetrievalMode,
  RetrievalQueryRequest,
  RetrievalQueryResponse,
} from "./retrieval";

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
  diagnosis?: {
    before_outcome: DiagnosisOutcome;
    after_outcome: DiagnosisOutcome;
    summary: string;
    resolved_failures: DiagnosisFailureCode[];
    introduced_failures: DiagnosisFailureCode[];
    gained_evidence: string[];
    lost_evidence: string[];
    gained_citations: string[];
    lost_citations: string[];
  } | null;
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
  diagnosis?: EvidenceDiagnosisSummary | null;
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

export function listTraces(signal?: AbortSignal): Promise<TraceSummary[]> {
  return requestJson<TraceSummary[]>("/api/v1/traces", { signal });
}

export function getTrace(
  traceId: string,
  signal?: AbortSignal,
): Promise<Trace> {
  return requestJson<Trace>(`/api/v1/traces/${traceId}`, { signal });
}

export function createTraceFromRetrievalRun(
  request: CreateTraceFromRetrievalRunRequest,
  signal?: AbortSignal,
): Promise<Trace> {
  return requestJson<Trace>(
    "/api/v1/traces/from-retrieval-run",
    jsonRequest("POST", request, signal),
  );
}

export function rerunTrace(
  traceId: string,
  request: RerunTraceRequest,
  signal?: AbortSignal,
): Promise<TraceRerunResponse> {
  return requestJson<TraceRerunResponse>(
    `/api/v1/traces/${traceId}/rerun`,
    jsonRequest("POST", request, signal),
  );
}
