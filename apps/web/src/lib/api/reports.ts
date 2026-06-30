import { jsonRequest, requestJson, requestText } from "./client";

export type DebugReportPrivacyMode =
  | "metadata_only"
  | "snippets_allowed"
  | "full_local_only";

export type DebugReportSource =
  | { type: "trace"; trace_id: string }
  | { type: "eval_experiment"; experiment_id: string }
  | { type: "ci_eval_run"; run_id: string }
  | { type: "manual"; label: string };

export type DebugReportSeverity = "info" | "warning" | "critical";
export type DebugReportRecommendationPriority =
  | "critical"
  | "high"
  | "medium"
  | "low";
export type DebugReportRecommendationArea =
  | "chunking"
  | "embeddings"
  | "top_k"
  | "retrieval_mode"
  | "reranking"
  | "metadata_filters"
  | "citations"
  | "corpus_coverage"
  | "other";
export type DebugReportEvidenceRole = "retrieved" | "expected" | "missing";

export interface DebugReportFinding {
  code: string;
  severity: DebugReportSeverity;
  title: string;
  summary: string;
  failure_labels: string[];
  evidence_refs: string[];
}

export interface DebugReportRecommendation {
  code: string;
  priority: DebugReportRecommendationPriority;
  area: DebugReportRecommendationArea;
  title: string;
  rationale: string;
  action: string;
  finding_codes: string[];
}

export interface DebugReportEvidenceRef {
  label: string;
  role: DebugReportEvidenceRole;
  source_id: string | null;
  document_id: string | null;
  chunk_id: string | null;
  rank: number | null;
  document_path: string | null;
  section_title: string | null;
  checksum_prefix: string | null;
  citation_label: string | null;
  snippet: string | null;
  evidence_strength: string | null;
  chunk_quality_flags: string[];
  retrieval_quality_flags: string[];
}

export interface DebugReport {
  id: string;
  workspace_id: string;
  project_id: string;
  title: string;
  subject: string;
  source: DebugReportSource;
  privacy_mode: DebugReportPrivacyMode;
  executive_summary: string;
  context: Record<string, string>;
  findings: DebugReportFinding[];
  recommendations: DebugReportRecommendation[];
  evidence: DebugReportEvidenceRef[];
  created_at: string;
}

export interface CreateDebugReportRequest {
  privacy_mode?: DebugReportPrivacyMode;
}

export function listDebugReports(signal?: AbortSignal): Promise<DebugReport[]> {
  return requestJson<DebugReport[]>("/api/v1/reports", { signal });
}

export function getDebugReport(
  reportId: string,
  signal?: AbortSignal,
): Promise<DebugReport> {
  return requestJson<DebugReport>(`/api/v1/reports/${reportId}`, { signal });
}

export function createDebugReportFromTrace(
  traceId: string,
  request: CreateDebugReportRequest = {},
  signal?: AbortSignal,
): Promise<DebugReport> {
  return requestJson<DebugReport>(
    "/api/v1/reports/from-trace",
    jsonRequest("POST", { trace_id: traceId, ...request }, signal),
    [201],
  );
}

export function createDebugReportFromExperiment(
  experimentId: string,
  request: CreateDebugReportRequest = {},
  signal?: AbortSignal,
): Promise<DebugReport> {
  return requestJson<DebugReport>(
    "/api/v1/reports/from-experiment",
    jsonRequest("POST", { experiment_id: experimentId, ...request }, signal),
    [201],
  );
}

export function createDebugReportFromCiRun(
  runId: string,
  request: CreateDebugReportRequest = {},
  signal?: AbortSignal,
): Promise<DebugReport> {
  return requestJson<DebugReport>(
    "/api/v1/reports/from-ci-run",
    jsonRequest("POST", { run_id: runId, ...request }, signal),
    [201],
  );
}

export function exportDebugReportMarkdown(
  reportId: string,
  signal?: AbortSignal,
): Promise<string> {
  return requestText(`/api/v1/reports/${reportId}/export.md`, { signal });
}
