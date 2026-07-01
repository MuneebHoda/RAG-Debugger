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
