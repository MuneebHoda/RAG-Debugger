import type { FailureLabel } from "../../../../lib/api/traces";

export const FAILURE_LABELS: Record<FailureLabel, string> = {
  missing_document: "Expected information may be missing from the corpus.",
  bad_chunking: "The relevant text may be split into weak evidence units.",
  bad_embedding: "Semantic indexing may not represent this evidence well.",
  bad_ranking: "Relevant evidence exists but ranked too low.",
  bad_prompt:
    "The answer instructions may not use the retrieved evidence correctly.",
  unsupported_question: "The corpus does not appear to support this question.",
  hallucinated_answer: "The answer contains claims not supported by citations.",
  weak_evidence: "Retrieved evidence is too weak for a confident answer.",
  missing_embedding_index:
    "Some chunks are not available to semantic retrieval.",
  duplicate_evidence: "Repeated evidence is crowding out distinct results.",
  heading_only_evidence: "A heading ranked without enough supporting content.",
};

export type TraceRecommendation = {
  label: string;
  detail: string;
  route: string;
};

export function recommendationFor(labels: FailureLabel[]): TraceRecommendation {
  if (
    labels.includes("missing_document") ||
    labels.includes("unsupported_question")
  ) {
    return {
      label: "Review Corpus",
      detail: "Confirm the supporting document is present and readable.",
      route: "/app/sources",
    };
  }
  if (
    labels.includes("bad_embedding") ||
    labels.includes("missing_embedding_index")
  ) {
    return {
      label: "Review indexing",
      detail: "Refresh embeddings before comparing semantic retrieval.",
      route: "/app/retrieval",
    };
  }
  if (labels.length > 0) {
    return {
      label: "Compare retrieval",
      detail:
        "Rerun this question with another ranking mode and compare evidence.",
      route: "?tab=compare",
    };
  }
  return {
    label: "Add quality case",
    detail: "Preserve this successful result as regression coverage.",
    route: "?tab=summary#quality",
  };
}
