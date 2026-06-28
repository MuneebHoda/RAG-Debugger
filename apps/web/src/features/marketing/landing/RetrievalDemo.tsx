import { CheckCircle2, FileSearch, Gauge, Search } from "lucide-react";
import { AnimatePresence, m, useReducedMotion } from "motion/react";
import { useState } from "react";

import {
  demoModes,
  demoQueries,
  type DemoMode,
  type DemoQuery,
} from "./landingData";
import { quickTransition, revealVariants, viewportOnce } from "./motion";
import styles from "./RetrievalDemo.module.css";

export function RetrievalDemo() {
  const [queryId, setQueryId] = useState<DemoQuery["id"]>("policy");
  const [modeId, setModeId] = useState<DemoMode["id"]>("hybrid");
  const reducedMotion = useReducedMotion();
  const query =
    demoQueries.find((item) => item.id === queryId) ?? demoQueries[0];
  const mode = demoModes.find((item) => item.id === modeId) ?? demoModes[2];

  return (
    <m.section
      className={styles.section}
      id="retrieval-demo"
      initial={reducedMotion ? false : "hidden"}
      variants={revealVariants}
      viewport={viewportOnce}
      whileInView="visible"
      aria-labelledby="retrieval-demo-title"
    >
      <header className={styles.heading}>
        <div>
          <p>Try the evidence layer</p>
          <h2 id="retrieval-demo-title">
            Change the retrieval strategy. See why it matters.
          </h2>
        </div>
        <span>
          Explore a deterministic product example. CorpusLab exposes the signals
          behind the answer instead of hiding them behind a confidence number.
        </span>
      </header>

      <div className={styles.tool}>
        <div className={styles.controls}>
          <div className={styles.controlLabel}>
            <Search aria-hidden="true" size={16} /> Example question
          </div>
          <div className={styles.queryOptions} aria-label="Example questions">
            {demoQueries.map((item) => (
              <button
                aria-pressed={item.id === query.id}
                className={
                  item.id === query.id ? styles.activeQuery : styles.query
                }
                key={item.id}
                type="button"
                onClick={() => setQueryId(item.id)}
              >
                {item.label}
              </button>
            ))}
          </div>
          <p className={styles.question}>{query.question}</p>

          <div className={styles.controlLabel}>
            <Gauge aria-hidden="true" size={16} /> Retrieval mode
          </div>
          <div className={styles.modeOptions} aria-label="Retrieval mode">
            {demoModes.map((item) => (
              <button
                aria-pressed={item.id === mode.id}
                className={
                  item.id === mode.id ? styles.activeMode : styles.mode
                }
                key={item.id}
                type="button"
                onClick={() => setModeId(item.id)}
              >
                {item.label}
              </button>
            ))}
          </div>

          <div className={styles.breakdown} aria-label="Score lineage">
            {mode.breakdown.map((signal) => (
              <div className={styles.scoreRow} key={signal.label}>
                <span>{signal.label}</span>
                <div className={styles.track}>
                  <m.i
                    animate={{ scaleX: signal.value / 100 }}
                    initial={false}
                    transition={
                      reducedMotion ? { duration: 0 } : quickTransition
                    }
                  />
                </div>
                <strong>{signal.value}</strong>
              </div>
            ))}
          </div>
        </div>

        <div className={styles.result} aria-live="polite">
          <div className={styles.resultMeta}>
            <span>{mode.label}</span>
            <span>{mode.latency} ms</span>
            <strong>{mode.score}% evidence strength</strong>
          </div>
          <AnimatePresence initial={false} mode="wait">
            <m.div
              animate={{ opacity: 1, y: 0 }}
              className={styles.evidence}
              exit={{ opacity: 0, y: reducedMotion ? 0 : -8 }}
              initial={{ opacity: 0, y: reducedMotion ? 0 : 8 }}
              key={`${query.id}-${mode.id}`}
              transition={reducedMotion ? { duration: 0 } : quickTransition}
            >
              <p className={styles.resultLabel}>Evidence summary</p>
              <h3>{query.evidence}</h3>
              <div className={styles.citation}>
                <FileSearch aria-hidden="true" size={17} />
                <span>
                  <small>Citation [1]</small>
                  <strong>{query.citation}</strong>
                </span>
              </div>
              <div className={styles.diagnosis}>
                <CheckCircle2 aria-hidden="true" size={18} />
                <span>{mode.diagnosis}</span>
              </div>
            </m.div>
          </AnimatePresence>
        </div>
      </div>
    </m.section>
  );
}
