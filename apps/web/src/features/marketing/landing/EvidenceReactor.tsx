import { m, type MotionValue, useReducedMotion } from "motion/react";

import type { CommandCenterScenario } from "./commandCenterData";
import { EvidenceReactorDeck } from "./EvidenceReactorDeck";
import { EvidenceReactorScene } from "./EvidenceReactorScene";
import {
  deriveEvidenceReactorState,
  type EvidenceReactorGate,
} from "./evidenceReactorState";
import styles from "./EvidenceReactor.module.css";

type EvidenceReactorProps = {
  scenario: CommandCenterScenario;
  parallaxX: MotionValue<number>;
  parallaxY: MotionValue<number>;
};

export function EvidenceReactor({
  scenario,
  parallaxX,
  parallaxY,
}: EvidenceReactorProps) {
  const reducedMotion = Boolean(useReducedMotion());
  const state = deriveEvidenceReactorState(scenario);
  const gateColor = reactorTone(state.gate);
  const parallax = {
    x: reducedMotion ? 0 : parallaxX,
    y: reducedMotion ? 0 : parallaxY,
  };

  return (
    <>
      <m.div
        aria-hidden="true"
        className={styles.layer}
        data-evidence-reactor=""
        data-reactor-candidates={state.candidateCount}
        data-reactor-coverage={state.coverage}
        data-reactor-gate={state.gate}
        data-reactor-motion={reducedMotion ? "static" : "active"}
        data-reactor-outcome={state.outcome}
        data-reactor-report={state.reportStatus}
        data-reactor-supported={state.supportedCount}
        style={parallax}
      >
        <EvidenceReactorScene
          gateColor={gateColor}
          reducedMotion={reducedMotion}
          scenario={scenario}
          state={state}
        />
      </m.div>

      <m.div aria-hidden="true" className={styles.deckLayer} style={parallax}>
        <EvidenceReactorDeck
          gateColor={gateColor}
          reducedMotion={reducedMotion}
          state={state}
        />
      </m.div>
    </>
  );
}

function reactorTone(gate: EvidenceReactorGate) {
  if (gate === "passed") return "#d5ff5f";
  if (gate === "review") return "#f4bd75";
  return "#ff8f84";
}
