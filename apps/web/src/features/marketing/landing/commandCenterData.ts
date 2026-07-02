export type CommandCenterOutcome = "strong" | "mixed" | "weak" | "failing";

export type EvidenceSignal = {
  label: "Lexical" | "Semantic" | "Metadata";
  value: number;
};

export type CommandCenterEvidence = {
  id: string;
  title: string;
  reference: string;
  excerpt: string;
  score: number;
  support: "supported" | "candidate";
  supportLabel: string;
  signals: EvidenceSignal[];
};

export type CommandCenterScenario = {
  id: CommandCenterOutcome;
  label: string;
  outcomeLabel: string;
  summary: string;
  query: string;
  latencyMs: number;
  coverage: number;
  answerability: string;
  failureLabels: string[];
  recommendation: string;
  gate: "Passed" | "Review" | "Failed";
  reportStatus: string;
  evidence: CommandCenterEvidence[];
};

const query =
  "How long does a password reset link remain valid, and what duplicated evidence could confuse the answer?";

export const commandCenterScenarios: CommandCenterScenario[] = [
  {
    id: "failing",
    label: "Failing",
    outcomeLabel: "Answerability failed",
    summary:
      "Candidates ranked, but none directly support the requested validity period.",
    query,
    latencyMs: 14,
    coverage: 0,
    answerability: "Insufficient evidence",
    failureLabels: ["answerability_gap", "semantic_only_match"],
    recommendation:
      "Add or recover the current reset-policy evidence before changing ranking weights.",
    gate: "Failed",
    reportStatus: "Diagnosis ready",
    evidence: [
      {
        id: "legacy-faq",
        title: "Legacy support FAQ",
        reference: "support-faq-2024.md · chunk 18",
        excerpt:
          "Account recovery links are issued after identity verification completes.",
        score: 88,
        support: "candidate",
        supportLabel: "Semantic only",
        signals: [
          { label: "Lexical", value: 42 },
          { label: "Semantic", value: 94 },
          { label: "Metadata", value: 71 },
        ],
      },
      {
        id: "security-guide",
        title: "Security operations",
        reference: "security-guide.md · chunk 7",
        excerpt:
          "Reset credentials immediately after suspicious account activity.",
        score: 79,
        support: "candidate",
        supportLabel: "Off-topic body",
        signals: [
          { label: "Lexical", value: 54 },
          { label: "Semantic", value: 82 },
          { label: "Metadata", value: 64 },
        ],
      },
      {
        id: "account-heading",
        title: "Account recovery",
        reference: "identity-handbook.pdf · chunk 31",
        excerpt: "Password reset links",
        score: 72,
        support: "candidate",
        supportLabel: "Heading only",
        signals: [
          { label: "Lexical", value: 89 },
          { label: "Semantic", value: 69 },
          { label: "Metadata", value: 58 },
        ],
      },
    ],
  },
  {
    id: "weak",
    label: "Weak",
    outcomeLabel: "Evidence recovered, rank weak",
    summary:
      "One chunk supports the answer, but it appears below two broad candidates.",
    query,
    latencyMs: 16,
    coverage: 33,
    answerability: "Supported at rank 3",
    failureLabels: ["weak_evidence", "low_score_margin"],
    recommendation:
      "Increase retrieval depth, then inspect why direct policy evidence ranks third.",
    gate: "Review",
    reportStatus: "2 findings ready",
    evidence: [
      {
        id: "legacy-faq",
        title: "Legacy support FAQ",
        reference: "support-faq-2024.md · chunk 18",
        excerpt:
          "Account recovery links are issued after identity verification completes.",
        score: 78,
        support: "candidate",
        supportLabel: "Candidate only",
        signals: [
          { label: "Lexical", value: 56 },
          { label: "Semantic", value: 86 },
          { label: "Metadata", value: 70 },
        ],
      },
      {
        id: "security-guide",
        title: "Security operations",
        reference: "security-guide.md · chunk 7",
        excerpt:
          "Reset credentials immediately after suspicious account activity.",
        score: 75,
        support: "candidate",
        supportLabel: "Candidate only",
        signals: [
          { label: "Lexical", value: 62 },
          { label: "Semantic", value: 79 },
          { label: "Metadata", value: 61 },
        ],
      },
      {
        id: "current-policy",
        title: "Current recovery policy",
        reference: "account-recovery.md · chunk 4",
        excerpt:
          "Password reset links remain valid for 20 minutes and expire after first use.",
        score: 71,
        support: "supported",
        supportLabel: "Direct body support",
        signals: [
          { label: "Lexical", value: 92 },
          { label: "Semantic", value: 84 },
          { label: "Metadata", value: 45 },
        ],
      },
    ],
  },
  {
    id: "mixed",
    label: "Mixed",
    outcomeLabel: "Answer supported, corpus mixed",
    summary:
      "Direct evidence answers the question, while a stale duplicate remains in the candidate set.",
    query,
    latencyMs: 12,
    coverage: 67,
    answerability: "Supported at rank 1",
    failureLabels: ["duplicate_evidence", "stale_candidate"],
    recommendation:
      "Keep the supported answer and remove or supersede the stale duplicate before release.",
    gate: "Review",
    reportStatus: "1 finding ready",
    evidence: [
      {
        id: "current-policy",
        title: "Current recovery policy",
        reference: "account-recovery.md · chunk 4",
        excerpt:
          "Password reset links remain valid for 20 minutes and expire after first use.",
        score: 94,
        support: "supported",
        supportLabel: "Direct body support",
        signals: [
          { label: "Lexical", value: 96 },
          { label: "Semantic", value: 93 },
          { label: "Metadata", value: 72 },
        ],
      },
      {
        id: "duplicate-policy",
        title: "Superseded recovery policy",
        reference: "account-recovery-legacy.md · chunk 4",
        excerpt:
          "Password reset links remain active for 30 minutes unless manually revoked.",
        score: 83,
        support: "supported",
        supportLabel: "Stale duplicate",
        signals: [
          { label: "Lexical", value: 91 },
          { label: "Semantic", value: 88 },
          { label: "Metadata", value: 48 },
        ],
      },
      {
        id: "security-guide",
        title: "Security operations",
        reference: "security-guide.md · chunk 7",
        excerpt:
          "Reset credentials immediately after suspicious account activity.",
        score: 58,
        support: "candidate",
        supportLabel: "Candidate only",
        signals: [
          { label: "Lexical", value: 44 },
          { label: "Semantic", value: 70 },
          { label: "Metadata", value: 36 },
        ],
      },
    ],
  },
  {
    id: "strong",
    label: "Strong",
    outcomeLabel: "Direct evidence, release ready",
    summary:
      "The current policy ranks first, citations agree, and stale evidence is excluded.",
    query,
    latencyMs: 9,
    coverage: 100,
    answerability: "Supported at rank 1",
    failureLabels: [],
    recommendation:
      "Save this run as the regression baseline for the account-recovery dataset.",
    gate: "Passed",
    reportStatus: "Audit ready",
    evidence: [
      {
        id: "current-policy",
        title: "Current recovery policy",
        reference: "account-recovery.md · chunk 4",
        excerpt:
          "Password reset links remain valid for 20 minutes and expire after first use.",
        score: 97,
        support: "supported",
        supportLabel: "Direct body support",
        signals: [
          { label: "Lexical", value: 98 },
          { label: "Semantic", value: 95 },
          { label: "Metadata", value: 81 },
        ],
      },
      {
        id: "identity-controls",
        title: "Identity controls",
        reference: "identity-handbook.pdf · chunk 32",
        excerpt:
          "Recovery links are invalidated after successful password rotation.",
        score: 84,
        support: "supported",
        supportLabel: "Corroborating evidence",
        signals: [
          { label: "Lexical", value: 79 },
          { label: "Semantic", value: 89 },
          { label: "Metadata", value: 68 },
        ],
      },
      {
        id: "support-runbook",
        title: "Support runbook",
        reference: "support-runbook.md · chunk 12",
        excerpt:
          "Agents should direct customers to request a new link after expiration.",
        score: 69,
        support: "candidate",
        supportLabel: "Supporting context",
        signals: [
          { label: "Lexical", value: 61 },
          { label: "Semantic", value: 76 },
          { label: "Metadata", value: 53 },
        ],
      },
    ],
  },
];

export function getCommandCenterScenario(
  id: CommandCenterOutcome,
): CommandCenterScenario {
  return (
    commandCenterScenarios.find((scenario) => scenario.id === id) ??
    commandCenterScenarios[0]
  );
}
