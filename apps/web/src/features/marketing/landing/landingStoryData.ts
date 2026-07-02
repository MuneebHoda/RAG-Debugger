import {
  Blocks,
  BrainCircuit,
  FileSearch,
  ScanSearch,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";

export type DiagnosticStageId =
  | "missing_source"
  | "broken_chunk"
  | "stale_embeddings"
  | "ranking_drift"
  | "unsupported_answer";

export type DiagnosticEvidence = {
  label: string;
  score: number | null;
  support: "supported" | "candidate_only" | "missing";
  signal: string;
};

export type DiagnosticFailureStage = {
  id: DiagnosticStageId;
  label: string;
  title: string;
  summary: string;
  outcome: "mixed" | "weak" | "failing";
  failureLabels: string[];
  recommendation: string;
  evidence: DiagnosticEvidence[];
  icon: LucideIcon;
};

export type QualityLoopStep = {
  index: string;
  label: string;
  title: string;
  detail: string;
};

export type ExperimentModeResult = {
  mode: "Lexical" | "Vector" | "Hybrid";
  recall: string;
  precision: string;
  mrr: string;
  latency: string;
  status: "Pass" | "Fail";
};

export const diagnosticStages: DiagnosticFailureStage[] = [
  {
    id: "missing_source",
    label: "Missing source",
    title: "The answer never entered the corpus.",
    summary:
      "Retrieval found adjacent support content, but the expected recovery policy is absent from the indexed source set.",
    outcome: "failing",
    failureLabels: ["missing_expected_evidence", "corpus_coverage_gap"],
    recommendation:
      "Ingest the expected policy source, verify extraction, and rerun the same question before tuning retrieval.",
    evidence: [
      {
        label: "Expected recovery policy",
        score: null,
        support: "missing",
        signal: "not indexed",
      },
      {
        label: "security-operations · chunk 7",
        score: 73,
        support: "candidate_only",
        signal: "semantic only",
      },
      {
        label: "support-routing · chunk 11",
        score: 62,
        support: "candidate_only",
        signal: "metadata match",
      },
    ],
    icon: FileSearch,
  },
  {
    id: "broken_chunk",
    label: "Chunk boundary",
    title: "The qualifier was split from the answer.",
    summary:
      "The reset procedure and its identity requirement landed in separate chunks, leaving both pieces too weak to cite alone.",
    outcome: "weak",
    failureLabels: ["weak_evidence", "broken_context_boundary"],
    recommendation:
      "Rechunk the policy with structured boundaries and keep the recovery requirement beside the procedure it qualifies.",
    evidence: [
      {
        label: "account-recovery · chunk 14",
        score: 81,
        support: "candidate_only",
        signal: "procedure only",
      },
      {
        label: "account-recovery · chunk 15",
        score: 76,
        support: "candidate_only",
        signal: "qualifier only",
      },
      {
        label: "security-operations · chunk 7",
        score: 58,
        support: "candidate_only",
        signal: "off topic",
      },
    ],
    icon: Blocks,
  },
  {
    id: "stale_embeddings",
    label: "Stale index",
    title: "Updated evidence is missing from semantic search.",
    summary:
      "The current chunk checksum no longer matches the stored embedding snapshot, so hybrid retrieval is evaluating stale meaning.",
    outcome: "failing",
    failureLabels: ["missing_embeddings", "stale_embedding_index"],
    recommendation:
      "Reindex the affected source with the active model and confirm every current chunk checksum is represented.",
    evidence: [
      {
        label: "account-recovery · chunk 9",
        score: null,
        support: "missing",
        signal: "embedding stale",
      },
      {
        label: "legacy-support · chunk 4",
        score: 69,
        support: "candidate_only",
        signal: "old semantic match",
      },
      {
        label: "security-operations · chunk 7",
        score: 64,
        support: "candidate_only",
        signal: "lexical only",
      },
    ],
    icon: BrainCircuit,
  },
  {
    id: "ranking_drift",
    label: "Ranking drift",
    title: "The correct evidence exists, but ranks below noise.",
    summary:
      "Lexical and vector signals disagree, while two heading-heavy candidates crowd the supported policy below the answer cutoff.",
    outcome: "mixed",
    failureLabels: ["vector_lexical_disagreement", "low_score_margin"],
    recommendation:
      "Compare lexical, vector, and hybrid runs, suppress heading-only chunks, then validate the new rank in Eval Lab.",
    evidence: [
      {
        label: "security-operations · chunk 7",
        score: 88,
        support: "candidate_only",
        signal: "heading boost",
      },
      {
        label: "support-routing · chunk 11",
        score: 84,
        support: "candidate_only",
        signal: "semantic lead",
      },
      {
        label: "account-recovery · chunk 9",
        score: 82,
        support: "supported",
        signal: "direct body support",
      },
    ],
    icon: ScanSearch,
  },
  {
    id: "unsupported_answer",
    label: "Unsupported answer",
    title: "Candidates ranked. None can safely answer.",
    summary:
      "Paths, metadata, and semantic similarity produced plausible candidates, but no chunk body directly supports the requested recovery rule.",
    outcome: "failing",
    failureLabels: ["answerability_gap", "semantic_only_match"],
    recommendation:
      "Keep the abstention, inspect corpus coverage, and add this question to Eval Lab with explicit expected evidence.",
    evidence: [
      {
        label: "legacy-support · chunk 4",
        score: 88,
        support: "candidate_only",
        signal: "semantic only",
      },
      {
        label: "security-operations · chunk 7",
        score: 79,
        support: "candidate_only",
        signal: "path only",
      },
      {
        label: "account-recovery · chunk 2",
        score: 72,
        support: "candidate_only",
        signal: "heading only",
      },
    ],
    icon: ShieldCheck,
  },
];

export const qualityLoopSteps: QualityLoopStep[] = [
  {
    index: "01",
    label: "Trace",
    title: "Save the failed run.",
    detail: "Freeze its query, ranked evidence, labels, and score explanation.",
  },
  {
    index: "02",
    label: "Eval case",
    title: "Name the expected evidence.",
    detail: "Turn the observed failure into a deterministic regression case.",
  },
  {
    index: "03",
    label: "Experiment",
    title: "Compare modes and config.",
    detail:
      "Run lexical, vector, and hybrid against the same dataset snapshot.",
  },
  {
    index: "04",
    label: "Gate",
    title: "Block the unsafe change.",
    detail: "Fail release criteria when evidence quality moves backward.",
  },
];

export const experimentResults: ExperimentModeResult[] = [
  {
    mode: "Lexical",
    recall: "0.60",
    precision: "0.40",
    mrr: "0.50",
    latency: "8 ms",
    status: "Fail",
  },
  {
    mode: "Vector",
    recall: "0.80",
    precision: "0.53",
    mrr: "0.75",
    latency: "14 ms",
    status: "Pass",
  },
  {
    mode: "Hybrid",
    recall: "0.67",
    precision: "0.44",
    mrr: "0.50",
    latency: "12 ms",
    status: "Fail",
  },
];

export const privacyBoundary = {
  local: ["Raw documents", "Chunk text + embeddings", "Full local traces"],
  approved: [
    "Opaque evidence IDs",
    "Scores, labels, and metrics",
    "Metadata-only audit reports",
  ],
} as const;
