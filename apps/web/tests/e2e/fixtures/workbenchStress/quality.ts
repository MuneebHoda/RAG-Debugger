import type { RetrievalEvalExperiment } from "../../../../src/lib/api/evalLab";

import { stressIds, stressValues, unbrokenToken } from "./shared";

export const stressExperiment = {
  id: stressIds.experiment,
  dataset_id: stressIds.dataset,
  dataset_name: `Production corpus gate ${unbrokenToken}`,
  name: `Release retrieval gate ${unbrokenToken}`,
  modes: ["hybrid", "vector", "lexical"],
  top_k: 25,
  config_snapshot: {
    top_k: 25,
    scoring_weights: {
      semantic: 0.45,
      lexical: 0.3,
      phrase: 0.1,
      section: 0.05,
      path: 0.05,
      metadata: 0.05,
    },
    embedding_model: {
      provider: "local",
      model_name: `local-hash-v1-${unbrokenToken}`,
      dimension: 384,
    },
    dataset_case_count: 1,
  },
  mode_results: ["hybrid", "vector", "lexical"].map((retrieval_mode) => ({
    retrieval_mode,
    case_count: 1,
    passed_count: 0,
    average_recall_at_k: 0.25,
    average_precision_at_k: 0.04,
    mean_reciprocal_rank: 0.2,
    citation_coverage: 0,
    weak_evidence_count: 1,
    missing_embedding_failures: 1,
    latency_p50_ms: 12_345,
    latency_p95_ms: 23_456,
    case_results: [],
  })),
  comparison: {
    best_mode: "hybrid",
    mode_count: 3,
    recall_delta: 0.25,
    precision_delta: 0.04,
    latency_delta_ms: 11_111,
    summary: `Hybrid leads, but all modes remain below the release gate. ${unbrokenToken}`,
  },
  gate: {
    status: "failed",
    average_recall_at_k: 0.25,
    weak_evidence_rate: 1,
    critical_failure_count: 1,
    recall_threshold: 0.8,
    weak_evidence_limit: 0.2,
    reasons: [`Average recall is below 80%. ${unbrokenToken}`],
  },
  failures: [
    {
      case_id: stressIds.case,
      query: stressValues.query,
      retrieval_mode: "hybrid",
      label: "expected_evidence_missing",
      severity: "critical",
      message: `Expected evidence was not retrieved. ${unbrokenToken}`,
      top_hit_rank: null,
    },
  ],
  created_at: "2026-07-04T08:00:00Z",
} satisfies RetrievalEvalExperiment;
