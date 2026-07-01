import { CheckCircle2, FileCheck2, ShieldAlert, Sparkles } from "lucide-react";
import { AnimatePresence, m, useReducedMotion } from "motion/react";

import type { CommandCenterScenario } from "./commandCenterData";
import { quickTransition } from "./motion";
import styles from "./HeroDiagnosisCard.module.css";

type HeroDiagnosisCardProps = {
  scenario: CommandCenterScenario;
};

export function HeroDiagnosisCard({ scenario }: HeroDiagnosisCardProps) {
  const reducedMotion = useReducedMotion();

  return (
    <section className={styles.panel} aria-labelledby="hero-diagnosis-title">
      <header>
        <div>
          <span>Debugger</span>
          <h2 id="hero-diagnosis-title">Primary diagnosis</h2>
        </div>
        <OutcomeBadge outcome={scenario.id} label={scenario.label} />
      </header>

      <AnimatePresence initial={false} mode="wait">
        <m.div
          animate={{ opacity: 1, y: 0 }}
          className={styles.diagnosis}
          exit={{ opacity: 0, y: reducedMotion ? 0 : -8 }}
          initial={{ opacity: 0, y: reducedMotion ? 0 : 8 }}
          key={scenario.id}
          transition={reducedMotion ? { duration: 0 } : quickTransition}
        >
          <strong>{scenario.outcomeLabel}</strong>
          <p>{scenario.summary}</p>

          <div className={styles.labels} aria-label="Failure labels">
            {scenario.failureLabels.length > 0 ? (
              scenario.failureLabels.map((label) => (
                <span key={label}>{label.replaceAll("_", " ")}</span>
              ))
            ) : (
              <span className={styles.clearLabel}>
                <CheckCircle2 aria-hidden="true" size={12} /> No blocking labels
              </span>
            )}
          </div>

          <div className={styles.recommendation}>
            <Sparkles aria-hidden="true" size={15} />
            <span>
              <small>Recommended next action</small>
              <strong>{scenario.recommendation}</strong>
            </span>
          </div>
        </m.div>
      </AnimatePresence>

      <div className={styles.delivery}>
        <span>
          <ShieldAlert aria-hidden="true" size={14} /> CI gate
          <strong data-gate={scenario.gate.toLowerCase()}>
            {scenario.gate}
          </strong>
        </span>
        <span>
          <FileCheck2 aria-hidden="true" size={14} /> Audit report
          <strong>{scenario.reportStatus}</strong>
        </span>
      </div>
    </section>
  );
}

function OutcomeBadge({
  outcome,
  label,
}: {
  outcome: CommandCenterScenario["id"];
  label: string;
}) {
  return (
    <span className={styles.outcome} data-outcome={outcome}>
      <i aria-hidden="true" /> {label}
    </span>
  );
}
