import { requestJson } from "./client";
import type { EmbeddingStatus } from "./embeddings";
import type { RetrievalEvalGateStatus } from "./evalLab";
import type { RetrievalMode } from "./retrieval";
import type { DocumentProfile } from "./sources";

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

export function getOverview(signal?: AbortSignal): Promise<OverviewResponse> {
  return requestJson<OverviewResponse>("/api/v1/overview", { signal });
}
