import type {
  RetrievalEvalExperiment,
  RetrievalEvalExperimentSummary,
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

function sortedModes(modes: string[]) {
  return [...modes].sort().join(",");
}
