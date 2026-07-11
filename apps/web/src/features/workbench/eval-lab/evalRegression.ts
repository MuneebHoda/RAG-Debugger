import type {
  RetrievalEvalExperiment,
  RetrievalEvalExperimentSummary,
  RetrievalEvalRegressionComparison,
  RetrievalEvalRegressionMetric,
} from "../../../lib/api/evalLab";

export interface BaselineCompatibility {
  level: "fully_compatible" | "partially_compatible" | "incompatible";
  label: string;
  reason: string;
}

export function findAutomaticBaseline(
  current: RetrievalEvalExperiment,
  experiments: RetrievalEvalExperimentSummary[],
) {
  return (
    experiments.find((candidate) => {
      const compatibility = classifyBaselineCompatibility(candidate, current);
      return compatibility.level === "fully_compatible";
    }) ?? null
  );
}

export function classifyBaselineCompatibility(
  candidate: RetrievalEvalExperimentSummary,
  current: RetrievalEvalExperiment,
): BaselineCompatibility {
  const candidateTime = Date.parse(candidate.created_at);
  const currentTime = Date.parse(current.created_at);
  if (candidate.id === current.id) {
    return {
      level: "incompatible",
      label: "incompatible",
      reason: "The current experiment cannot be compared with itself.",
    };
  }
  if (Number.isFinite(candidateTime) && Number.isFinite(currentTime)) {
    if (candidateTime >= currentTime) {
      return {
        level: "incompatible",
        label: "incompatible",
        reason: "Only earlier experiments can be used as baselines.",
      };
    }
  }

  const sameTopK = candidate.top_k === current.top_k;
  const sameModes = sortedModes(candidate.modes) === sortedModes(current.modes);
  if (sameTopK && sameModes) {
    return {
      level: "fully_compatible",
      label: "fully compatible",
      reason: "Same dataset, top_k, and retrieval modes.",
    };
  }

  return {
    level: "partially_compatible",
    label: "partially compatible",
    reason: [
      sameTopK
        ? null
        : `top_k changed from ${candidate.top_k} to ${current.top_k}`,
      sameModes
        ? null
        : `modes changed from ${candidate.modes.join(", ")} to ${current.modes.join(", ")}`,
    ]
      .filter(Boolean)
      .join("; "),
  };
}

export function summarizeExperimentForComparison(
  experiment: RetrievalEvalExperiment,
): RetrievalEvalExperimentSummary {
  const bestResult =
    experiment.mode_results.find(
      (result) => result.retrieval_mode === experiment.comparison.best_mode,
    ) ?? experiment.mode_results[0];

  return {
    id: experiment.id,
    dataset_id: experiment.dataset_id,
    dataset_name: experiment.dataset_name,
    name: experiment.name,
    modes: experiment.modes,
    top_k: experiment.top_k,
    best_mode: experiment.comparison.best_mode,
    gate_status: experiment.gate.status,
    average_recall_at_k: bestResult?.average_recall_at_k ?? 0,
    average_precision_at_k: bestResult?.average_precision_at_k ?? 0,
    mean_reciprocal_rank: bestResult?.mean_reciprocal_rank ?? 0,
    citation_coverage: bestResult?.citation_coverage ?? 0,
    weak_evidence_case_rate: bestResult
      ? bestResult.weak_evidence_count / Math.max(bestResult.case_count, 1)
      : 0,
    missing_embedding_failures: bestResult?.missing_embedding_failures ?? 0,
    latency_p50_ms: bestResult?.latency_p50_ms ?? 0,
    latency_p95_ms: bestResult?.latency_p95_ms ?? 0,
    failure_count: experiment.failures.length,
    created_at: experiment.created_at,
  };
}

export function regressionExplanation(
  regression: RetrievalEvalRegressionComparison,
) {
  if (!regression.baseline_experiment_id) {
    return "No comparable baseline exists yet for this experiment.";
  }

  if (regression.classification === "regressed") {
    if (
      regression.baseline_gate_status === "passed" &&
      regression.current_gate_status === "failed"
    ) {
      return "Classification is regressed because the release gate moved from passed to failed.";
    }

    if (regression.newly_failed_cases.length > 0) {
      return `Classification is regressed because ${regression.newly_failed_cases.length} case${plural(regression.newly_failed_cases.length)} newly failed.`;
    }

    const regressedDelta = regression.metric_deltas.find(
      (delta) => delta.classification === "regressed",
    );
    if (regressedDelta) {
      return metricDeltaExplanation(regressedDelta);
    }

    return "Classification is regressed because backend regression rules detected a quality decline.";
  }

  if (regression.classification === "improved") {
    if (
      regression.baseline_gate_status === "failed" &&
      regression.current_gate_status === "passed"
    ) {
      return "Classification is improved because the release gate moved from failed to passed.";
    }

    if (regression.recovered_cases.length > 0) {
      return `Classification is improved because ${regression.recovered_cases.length} case${plural(regression.recovered_cases.length)} recovered.`;
    }

    const improvedDelta = regression.metric_deltas.find(
      (delta) => delta.classification === "improved",
    );
    if (improvedDelta) {
      return metricDeltaExplanation(improvedDelta);
    }

    return "Classification is improved because backend regression rules detected a quality gain.";
  }

  return "Classification is unchanged because gate status, cases, and metric deltas stayed within thresholds.";
}

function sortedModes(modes: string[]) {
  return [...modes].sort().join(",");
}

function metricDeltaExplanation(
  delta: RetrievalEvalRegressionComparison["metric_deltas"][number],
) {
  return `Classification is ${delta.classification} because ${metricLabel(delta.metric)} changed from ${metricValue(delta.metric, delta.baseline ?? 0)} to ${metricValue(delta.metric, delta.current)}.`;
}

function metricValue(metric: RetrievalEvalRegressionMetric, value: number) {
  if (metric === "latency_p95_ms") return `${Math.round(value)} ms`;
  if (metric === "missing_embedding_failures") return String(Math.round(value));
  return percentage(value);
}

function metricLabel(metric: RetrievalEvalRegressionMetric) {
  return metric.replaceAll("_", " ");
}

function plural(count: number) {
  return count === 1 ? "" : "s";
}

function percentage(value: number) {
  return `${Math.round(value * 100)}%`;
}
