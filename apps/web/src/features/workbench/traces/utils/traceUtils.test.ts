import { describe, expect, it } from "vitest";

import type { TraceSummary } from "../../../../lib/api/traces";
import { filterRuns } from "./runFilters";
import { signedNumber } from "./traceFormatting";
import { recommendationFor } from "./traceLabels";

const strongRun = runSummary("strong", "GPU indexing", []);
const weakRun = runSummary("weak", "Policy exception", ["weak_evidence"]);

describe("trace utilities", () => {
  it("filters runs by normalized query and attention state", () => {
    expect(filterRuns([strongRun, weakRun], " policy ", "all")).toEqual([
      weakRun,
    ]);
    expect(filterRuns([strongRun, weakRun], "", "attention")).toEqual([
      weakRun,
    ]);
    expect(filterRuns([strongRun, weakRun], "", "strong")).toEqual([strongRun]);
  });

  it("chooses deterministic recommendations for failure categories", () => {
    expect(recommendationFor(["missing_document"]).route).toBe("/app/sources");
    expect(recommendationFor(["missing_embedding_index"]).route).toBe(
      "/app/retrieval",
    );
    expect(recommendationFor(["bad_ranking"]).route).toBe("?tab=compare");
    expect(recommendationFor([]).route).toBe("?tab=summary#quality");
  });

  it("formats signed comparison metrics", () => {
    expect(signedNumber(1.25)).toBe("+1.25");
    expect(signedNumber(-0.5)).toBe("-0.50");
  });
});

function runSummary(
  evidenceStrength: TraceSummary["evidence_strength"],
  query: string,
  failureLabels: TraceSummary["failure_labels"],
): TraceSummary {
  return {
    id: query,
    query,
    retrieval_mode: "hybrid",
    latency_ms: 8,
    evidence_strength: evidenceStrength,
    failure_labels: failureLabels,
    span_count: 1,
    rerun_count: 0,
    created_at: "2026-06-27T10:46:19Z",
  };
}
