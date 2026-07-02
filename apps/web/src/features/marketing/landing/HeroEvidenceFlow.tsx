import { CheckCircle2, CircleDotDashed } from "lucide-react";
import { m, useReducedMotion } from "motion/react";

import type { CommandCenterEvidence } from "./commandCenterData";
import { quickTransition } from "./motion";
import styles from "./HeroEvidenceFlow.module.css";

type HeroEvidenceFlowProps = {
  evidence: CommandCenterEvidence[];
  scenarioId: string;
};

export function HeroEvidenceFlow({
  evidence,
  scenarioId,
}: HeroEvidenceFlowProps) {
  const reducedMotion = useReducedMotion();

  return (
    <section className={styles.panel} aria-labelledby="hero-evidence-title">
      <header className={styles.header}>
        <div>
          <span>Retriever</span>
          <h2 id="hero-evidence-title">Ranked evidence</h2>
        </div>
        <strong>{evidence.length} candidates</strong>
      </header>

      <m.ol
        animate="visible"
        className={styles.list}
        initial="hidden"
        key={scenarioId}
        variants={{
          hidden: {},
          visible: {
            transition: reducedMotion
              ? { duration: 0 }
              : { staggerChildren: 0.08 },
          },
        }}
      >
        {evidence.map((item, index) => (
          <m.li
            className={styles.evidence}
            key={item.id}
            variants={{
              hidden: { opacity: 0, x: reducedMotion ? 0 : -12 },
              visible: {
                opacity: 1,
                x: 0,
                transition: reducedMotion ? { duration: 0 } : quickTransition,
              },
            }}
          >
            <span className={styles.rank}>{index + 1}</span>
            <div className={styles.body}>
              <div className={styles.titleRow}>
                <strong>{item.title}</strong>
                <span
                  className={
                    item.support === "supported"
                      ? styles.supported
                      : styles.candidate
                  }
                >
                  {item.support === "supported" ? (
                    <CheckCircle2 aria-hidden="true" size={12} />
                  ) : (
                    <CircleDotDashed aria-hidden="true" size={12} />
                  )}
                  {item.supportLabel}
                </span>
              </div>
              <small>{item.reference}</small>
              <p>{item.excerpt}</p>
              <div className={styles.signals}>
                {item.signals.map((signal) => (
                  <span key={signal.label}>
                    {signal.label} {signal.value}
                  </span>
                ))}
              </div>
            </div>
            <div className={styles.score}>
              <strong>{item.score}</strong>
              <span>score</span>
            </div>
          </m.li>
        ))}
      </m.ol>
    </section>
  );
}
