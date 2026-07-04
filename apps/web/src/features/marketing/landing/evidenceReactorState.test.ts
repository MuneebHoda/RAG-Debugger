import { describe, expect, it } from "vitest";

import { commandCenterScenarios } from "./commandCenterData";
import { deriveEvidenceReactorState } from "./evidenceReactorState";

describe("deriveEvidenceReactorState", () => {
  it.each([
    {
      outcome: "failing",
      coverage: 0,
      supported: 0,
      candidates: 3,
      gate: "failed",
      unsupported: true,
      duplicate: false,
    },
    {
      outcome: "weak",
      coverage: 33,
      supported: 1,
      candidates: 2,
      gate: "review",
      unsupported: false,
      duplicate: false,
    },
    {
      outcome: "mixed",
      coverage: 67,
      supported: 2,
      candidates: 1,
      gate: "review",
      unsupported: false,
      duplicate: true,
    },
    {
      outcome: "strong",
      coverage: 100,
      supported: 2,
      candidates: 1,
      gate: "passed",
      unsupported: false,
      duplicate: false,
    },
  ] as const)(
    "projects the $outcome scenario without inventing state",
    ({
      outcome,
      coverage,
      supported,
      candidates,
      gate,
      unsupported,
      duplicate,
    }) => {
      const scenario = commandCenterScenarios.find(
        (candidate) => candidate.id === outcome,
      );

      expect(scenario).toBeDefined();
      const state = deriveEvidenceReactorState(scenario!);

      expect(state).toMatchObject({
        outcome,
        coverage,
        supportedCount: supported,
        candidateCount: candidates,
        gate,
        hasUnsupportedBranch: unsupported,
        hasDuplicateBranch: duplicate,
        reportStatus: scenario!.reportStatus,
      });
    },
  );

  it("clamps coverage to a safe visual range", () => {
    const scenario = commandCenterScenarios[0];

    expect(
      deriveEvidenceReactorState({ ...scenario, coverage: 140 }).coverage,
    ).toBe(100);
    expect(
      deriveEvidenceReactorState({ ...scenario, coverage: -20 }).coverage,
    ).toBe(0);
  });
});
