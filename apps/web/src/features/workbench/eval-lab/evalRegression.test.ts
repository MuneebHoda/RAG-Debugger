import { describe, expect, it } from "vitest";

import type { RetrievalEvalRegressionComparison } from "../../../lib/api/evalLab";
import { regressionExplanation } from "./evalRegression";

describe("regressionExplanation", () => {
  it("explains regressed metrics instead of improved gate movement when classification is regressed", () => {
    expect(
      regressionExplanation(
        comparison({
          classification: "regressed",
          baseline_gate_status: "failed",
          current_gate_status: "passed",
          metric_deltas: [metric("recall_at_k", "regressed", 0.4, 0.8, -0.4)],
        }),
      ),
    ).toBe(
      "Classification is regressed because recall at k changed from 80% to 40%.",
    );
  });

  it("explains regressed metrics instead of recovered cases when classification is regressed", () => {
    expect(
      regressionExplanation(
        comparison({
          classification: "regressed",
          baseline_gate_status: "failed",
          current_gate_status: "failed",
          recovered_cases: [caseRegression("improved")],
          metric_deltas: [
            metric("precision_at_k", "regressed", 0.2, 0.6, -0.4),
          ],
        }),
      ),
    ).toBe(
      "Classification is regressed because precision at k changed from 60% to 20%.",
    );
  });

  it("explains an improved metric when classification is improved", () => {
    expect(
      regressionExplanation(
        comparison({
          classification: "improved",
          metric_deltas: [
            metric("mean_reciprocal_rank", "improved", 0.9, 0.5, 0.4),
          ],
        }),
      ),
    ).toBe(
      "Classification is improved because mean reciprocal rank changed from 50% to 90%.",
    );
  });

  it("uses the first metric matching the overall classification", () => {
    expect(
      regressionExplanation(
        comparison({
          classification: "improved",
          metric_deltas: [
            metric("recall_at_k", "regressed", 0.5, 0.8, -0.3),
            metric("citation_coverage", "improved", 0.9, 0.4, 0.5),
          ],
        }),
      ),
    ).toBe(
      "Classification is improved because citation coverage changed from 40% to 90%.",
    );
  });

  it("explains unchanged comparisons without implying movement", () => {
    expect(
      regressionExplanation(
        comparison({
          classification: "unchanged",
          metric_deltas: [metric("recall_at_k", "unchanged", 0.8, 0.8, 0)],
        }),
      ),
    ).toBe(
      "Classification is unchanged because gate status, cases, and metric deltas stayed within thresholds.",
    );
  });
});

function comparison(
  overrides: Partial<RetrievalEvalRegressionComparison> = {},
): RetrievalEvalRegressionComparison {
  return {
    current_experiment_id: "current",
    baseline_experiment_id: "baseline",
    classification: "unchanged",
    current_gate_status: "failed",
    baseline_gate_status: "passed",
    metric_deltas: [],
    newly_failed_cases: [],
    recovered_cases: [],
    changed_top_evidence_cases: [],
    changed_failure_label_cases: [],
    summary: "Comparison summary.",
    ...overrides,
  };
}

function metric(
  name: RetrievalEvalRegressionComparison["metric_deltas"][number]["metric"],
  classification: RetrievalEvalRegressionComparison["classification"],
  current: number,
  baseline: number,
  delta: number,
) {
  return {
    metric: name,
    current,
    baseline,
    delta,
    classification,
  };
}

function caseRegression(
  classification: RetrievalEvalRegressionComparison["classification"],
): RetrievalEvalRegressionComparison["recovered_cases"][number] {
  return {
    case_id: "case",
    retrieval_mode: "hybrid",
    query: "Which evidence changed?",
    classification,
    current_passed: classification === "improved",
    baseline_passed: classification !== "improved",
    current_top_hit_rank: 1,
    baseline_top_hit_rank: 2,
    current_retrieved_chunk_ids: ["chunk-current"],
    baseline_retrieved_chunk_ids: ["chunk-baseline"],
    current_failure_labels: [],
    baseline_failure_labels: ["expected_evidence_missing"],
  };
}
