import type {
  CommandCenterOutcome,
  CommandCenterScenario,
} from "./commandCenterData";

export type EvidenceReactorGate = "passed" | "review" | "failed";

export type EvidenceReactorState = {
  outcome: CommandCenterOutcome;
  coverage: number;
  supportedCount: number;
  candidateCount: number;
  gate: EvidenceReactorGate;
  hasDuplicateBranch: boolean;
  hasUnsupportedBranch: boolean;
  reportStatus: string;
};

export function deriveEvidenceReactorState(
  scenario: CommandCenterScenario,
): EvidenceReactorState {
  const supportedCount = scenario.evidence.filter(
    (evidence) => evidence.support === "supported",
  ).length;

  return {
    outcome: scenario.id,
    coverage: Math.max(0, Math.min(100, scenario.coverage)),
    supportedCount,
    candidateCount: scenario.evidence.length - supportedCount,
    gate: scenario.gate.toLowerCase() as EvidenceReactorGate,
    hasDuplicateBranch: scenario.failureLabels.includes("duplicate_evidence"),
    hasUnsupportedBranch:
      scenario.coverage === 0 ||
      scenario.failureLabels.includes("answerability_gap"),
    reportStatus: scenario.reportStatus,
  };
}
