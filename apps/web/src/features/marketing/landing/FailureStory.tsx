import { ArrowRight, TriangleAlert } from "lucide-react";
import { AnimatePresence, m, useReducedMotion } from "motion/react";
import { useState } from "react";

import { failureStages, type FailureStage } from "./landingData";
import { motionTransition, quickTransition } from "./motion";
import { useRovingTabs } from "./useRovingTabs";
import styles from "./FailureStory.module.css";

const failureStageIds = failureStages.map((stage) => stage.id);

export function FailureStory() {
  const [activeId, setActiveId] = useState<FailureStage["id"]>("extract");
  const reducedMotion = useReducedMotion();
  const { handleTabKeyDown, registerTab } = useRovingTabs({
    ids: failureStageIds,
    onSelect: setActiveId,
  });
  const activeStage =
    failureStages.find((stage) => stage.id === activeId) ?? failureStages[0];

  return (
    <section className={styles.section} aria-labelledby="failure-story-title">
      <div className={styles.inner}>
        <header className={styles.heading}>
          <p>One connected failure surface</p>
          <h2 id="failure-story-title">
            Find the failure before it becomes an answer.
          </h2>
          <span>
            Follow the evidence path from raw document to release gate. Every
            stage remains inspectable, comparable, and measurable.
          </span>
        </header>

        <div
          aria-label="RAG failure stages"
          className={styles.tabs}
          role="tablist"
        >
          {failureStages.map((stage, index) => (
            <button
              aria-controls="failure-stage-panel"
              aria-selected={stage.id === activeStage.id}
              className={
                stage.id === activeStage.id ? styles.activeTab : styles.tab
              }
              id={`failure-tab-${stage.id}`}
              key={stage.id}
              ref={registerTab(index)}
              role="tab"
              tabIndex={stage.id === activeStage.id ? 0 : -1}
              type="button"
              onClick={() => setActiveId(stage.id)}
              onKeyDown={(event) => handleTabKeyDown(event, index)}
            >
              <stage.icon aria-hidden="true" size={16} />
              {stage.label}
            </button>
          ))}
        </div>

        <div
          aria-labelledby={`failure-tab-${activeStage.id}`}
          className={styles.stage}
          id="failure-stage-panel"
          role="tabpanel"
        >
          <AnimatePresence initial={false} mode="wait">
            <m.div
              animate={{ opacity: 1, x: 0 }}
              className={styles.copy}
              exit={{ opacity: 0, x: reducedMotion ? 0 : -12 }}
              initial={{ opacity: 0, x: reducedMotion ? 0 : 12 }}
              key={activeStage.id}
              transition={reducedMotion ? { duration: 0 } : quickTransition}
            >
              <p className={styles.stageLabel}>{activeStage.label}</p>
              <h3>{activeStage.title}</h3>
              <p>{activeStage.description}</p>
              <div className={styles.diagnosis}>
                <TriangleAlert aria-hidden="true" size={18} />
                <span>
                  <small>Observed failure</small>
                  <strong>{activeStage.diagnosis}</strong>
                </span>
              </div>
              <div className={styles.action}>
                <ArrowRight aria-hidden="true" size={17} />
                <span>{activeStage.action}</span>
              </div>
            </m.div>
          </AnimatePresence>

          <AnimatePresence initial={false} mode="wait">
            <m.figure
              animate={{ opacity: 1, scale: 1 }}
              className={styles.media}
              exit={{ opacity: 0, scale: reducedMotion ? 1 : 0.985 }}
              initial={{ opacity: 0, scale: reducedMotion ? 1 : 1.015 }}
              key={activeStage.image}
              transition={reducedMotion ? { duration: 0 } : motionTransition}
            >
              <img
                alt={`${activeStage.label} failure diagnosis in CorpusLab`}
                loading="lazy"
                src={activeStage.image}
              />
            </m.figure>
          </AnimatePresence>
        </div>
      </div>
    </section>
  );
}
