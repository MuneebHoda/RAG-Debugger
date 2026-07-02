import { CirclePause, CirclePlay, Command } from "lucide-react";
import { m, useReducedMotion } from "motion/react";

import { AnimatedMetric } from "./AnimatedMetric";
import {
  commandCenterScenarios,
  type CommandCenterOutcome,
  type CommandCenterScenario,
} from "./commandCenterData";
import { HeroDiagnosisCard } from "./HeroDiagnosisCard";
import { HeroEvidenceFlow } from "./HeroEvidenceFlow";
import { HeroSignalGraph } from "./HeroSignalGraph";
import { motionTransition } from "./motion";
import styles from "./HeroCommandCenterSurface.module.css";

type HeroCommandCenterSurfaceProps = {
  scenario: CommandCenterScenario;
  isPlaying: boolean;
  onSelectScenario: (id: CommandCenterOutcome) => void;
  onTogglePlayback: () => void;
};

export function HeroCommandCenterSurface({
  scenario,
  isPlaying,
  onSelectScenario,
  onTogglePlayback,
}: HeroCommandCenterSurfaceProps) {
  const reducedMotion = useReducedMotion();

  return (
    <m.div
      animate={{ opacity: 1, y: 0 }}
      aria-label="Interactive RAG diagnosis simulation"
      className={styles.commandCenter}
      initial={{ opacity: 0, y: reducedMotion ? 0 : 24 }}
      transition={
        reducedMotion ? { duration: 0 } : { ...motionTransition, delay: 0.12 }
      }
    >
      <div className={styles.toolbar}>
        <div className={styles.runIdentity}>
          <Command aria-hidden="true" size={15} />
          <span>
            <small>Interactive product simulation</small>
            <strong>run_7F3A · account recovery</strong>
          </span>
        </div>
        <div aria-label="Diagnosis scenarios" className={styles.scenarios}>
          {commandCenterScenarios.map((item) => (
            <button
              aria-pressed={item.id === scenario.id}
              key={item.id}
              type="button"
              onClick={() => onSelectScenario(item.id)}
            >
              {item.label}
            </button>
          ))}
        </div>
        <button
          aria-label={isPlaying ? "Pause simulation" : "Play simulation"}
          className={styles.playback}
          title={isPlaying ? "Pause simulation" : "Play simulation"}
          type="button"
          onClick={onTogglePlayback}
        >
          {isPlaying ? (
            <CirclePause aria-hidden="true" size={18} />
          ) : (
            <CirclePlay aria-hidden="true" size={18} />
          )}
        </button>
      </div>

      <div className={styles.metrics}>
        <AnimatedMetric label="Mode" value="Hybrid" />
        <AnimatedMetric
          label="Outcome"
          tone={outcomeTone(scenario.id)}
          value={scenario.label}
        />
        <AnimatedMetric
          label="Answerability"
          tone={scenario.coverage > 0 ? "positive" : "critical"}
          value={scenario.answerability}
        />
        <AnimatedMetric label="Latency" value={`${scenario.latencyMs} ms`} />
      </div>

      <div className={styles.query}>
        <span>Query</span>
        <p>{scenario.query}</p>
        <strong>top_k 3</strong>
      </div>

      <div className={styles.workspace}>
        <HeroEvidenceFlow
          evidence={scenario.evidence}
          scenarioId={scenario.id}
        />
        <div className={styles.analysis}>
          <HeroDiagnosisCard scenario={scenario} />
          <HeroSignalGraph scenario={scenario} />
        </div>
      </div>

      <div className={styles.timeline} aria-label="Diagnosis event stream">
        <span>
          <i /> Query received
        </span>
        <span>
          <i /> Candidates ranked
        </span>
        <span>
          <i /> Body support checked
        </span>
        <span>
          <i /> Diagnosis generated
        </span>
        <strong>{scenario.gate} gate</strong>
      </div>
    </m.div>
  );
}

function outcomeTone(outcome: CommandCenterOutcome) {
  if (outcome === "strong") return "positive" as const;
  if (outcome === "failing") return "critical" as const;
  return "warning" as const;
}
