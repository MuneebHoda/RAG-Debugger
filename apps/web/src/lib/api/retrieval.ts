import { jsonRequest, requestJson } from "./client";
import type { ChunkPreview, DocumentRecord, SourceRecord } from "./sources";
import type { RetrievalEmbeddingStatus } from "./embeddings";
import type {
  CreateRetrievalEvalCaseRequest,
  RetrievalEvalCase,
  RetrievalEvalRun,
  RunRetrievalEvalRequest,
} from "./evalLab";

export type RetrievalMode = "lexical" | "vector" | "hybrid";
export type EvidenceStrength = "strong" | "medium" | "weak";
export type ExtractiveAnswerStatus = "answered" | "insufficient_evidence";
export type RetrievalQualityFlag =
  | "duplicate"
  | "heading_only"
  | "too_short"
  | "weak_evidence"
  | "semantic_match"
  | "exact_term_match"
  | "section_only_match";
export type DiagnosisOutcome = "strong" | "mixed" | "weak" | "failing";
export type DiagnosisSeverity = "info" | "warning" | "critical";
export type DiagnosisFailureCode =
  | "missing_document"
  | "missing_embedding_index"
  | "partial_embedding_index"
  | "weak_evidence"
  | "duplicate_evidence"
  | "heading_only_evidence"
  | "low_score_margin"
  | "vector_lexical_disagreement"
  | "citation_missing"
  | "top_result_not_cited"
  | "missing_expected_evidence";
export type DiagnosisRemediationArea =
  | "chunking"
  | "embeddings"
  | "top_k"
  | "retrieval_mode"
  | "reranking"
  | "metadata_filters"
  | "citations"
  | "corpus_coverage"
  | "other";
export type DiagnosisRecommendationPriority =
  | "critical"
  | "high"
  | "medium"
  | "low";
export type DiagnosisScoreSignal =
  | "semantic"
  | "lexical"
  | "phrase"
  | "section"
  | "path"
  | "metadata"
  | "none";

export interface DiagnosisFailure {
  code: DiagnosisFailureCode;
  severity: DiagnosisSeverity;
  title: string;
  summary: string;
  evidence_refs: string[];
}

export interface EvidenceScoreExplanation {
  evidence_ref: string;
  chunk_id: string;
  rank: number;
  final_score: number;
  score_delta_from_previous: number | null;
  score_delta_to_next: number | null;
  dominant_signal: DiagnosisScoreSignal;
  score_breakdown: RetrievalScoreBreakdown;
  normalized_score_breakdown: RetrievalScoreBreakdown;
  summary: string;
}

export interface DiagnosisRecommendation {
  code: string;
  priority: DiagnosisRecommendationPriority;
  area: DiagnosisRemediationArea;
  title: string;
  rationale: string;
  action: string;
  failure_codes: DiagnosisFailureCode[];
  evidence_refs: string[];
}

export interface EvidenceDiagnosisSummary {
  outcome: DiagnosisOutcome;
  summary: string;
  primary_issue: DiagnosisFailure | null;
  failures: DiagnosisFailure[];
  score_explanations: EvidenceScoreExplanation[];
  recommendations: DiagnosisRecommendation[];
}

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
  diagnosis?: EvidenceDiagnosisSummary | null;
}

export function queryRetrieval(
  request: RetrievalQueryRequest,
  signal?: AbortSignal,
): Promise<RetrievalQueryResponse> {
  return requestJson<RetrievalQueryResponse>(
    "/api/v1/retrieval/query",
    jsonRequest("POST", request, signal),
  );
}

export function listRetrievalEvalCases(
  signal?: AbortSignal,
): Promise<RetrievalEvalCase[]> {
  return requestJson<RetrievalEvalCase[]>("/api/v1/retrieval/evals", {
    signal,
  });
}

export function createRetrievalEvalCase(
  request: CreateRetrievalEvalCaseRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalCase> {
  return requestJson<RetrievalEvalCase>(
    "/api/v1/retrieval/evals",
    jsonRequest("POST", request, signal),
  );
}

export function runRetrievalEvals(
  request: RunRetrievalEvalRequest,
  signal?: AbortSignal,
): Promise<RetrievalEvalRun> {
  return requestJson<RetrievalEvalRun>(
    "/api/v1/retrieval/evals/run",
    jsonRequest("POST", request, signal),
  );
}
