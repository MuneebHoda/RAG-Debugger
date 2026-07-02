import { m, useReducedMotion } from "motion/react";

import type { CommandCenterScenario } from "./commandCenterData";
import { motionTransition } from "./motion";
import styles from "./HeroSignalGraph.module.css";

type HeroSignalGraphProps = {
  scenario: CommandCenterScenario;
};

export function HeroSignalGraph({ scenario }: HeroSignalGraphProps) {
  const reducedMotion = useReducedMotion();
  const supportedCount = scenario.evidence.filter(
    (item) => item.support === "supported",
  ).length;

  return (
    <section className={styles.panel} aria-labelledby="hero-lineage-title">
      <header>
        <span>Signal graph</span>
        <h2 id="hero-lineage-title">Evidence lineage</h2>
      </header>
      <div className={styles.graph}>
        <svg aria-hidden="true" viewBox="0 0 360 116">
          {[24, 58, 92].map((y, index) => (
            <m.path
              animate={{ pathLength: 1, opacity: 1 }}
              d={`M 20 ${y} C 105 ${y}, 105 58, 175 58 S 260 58, 338 58`}
              initial={{
                pathLength: reducedMotion ? 1 : 0,
                opacity: reducedMotion ? 1 : 0.2,
              }}
              key={`${scenario.id}-${y}`}
              transition={
                reducedMotion
                  ? { duration: 0 }
                  : { ...motionTransition, delay: index * 0.09 }
              }
            />
          ))}
          <circle cx="20" cy="24" r="5" />
          <circle cx="20" cy="58" r="5" />
          <circle cx="20" cy="92" r="5" />
          <circle className={styles.gateNode} cx="175" cy="58" r="8" />
          <circle className={styles.resultNode} cx="338" cy="58" r="9" />
        </svg>
        <div className={styles.labels}>
          <span>Candidates</span>
          <span>{supportedCount} supported</span>
          <strong>{scenario.gate}</strong>
        </div>
      </div>
      <div className={styles.coverage}>
        <span>Answer support coverage</span>
        <strong>{scenario.coverage}%</strong>
        <div>
          <m.i
            animate={{ scaleX: scenario.coverage / 100 }}
            initial={false}
            transition={reducedMotion ? { duration: 0 } : motionTransition}
          />
        </div>
      </div>
    </section>
  );
}
