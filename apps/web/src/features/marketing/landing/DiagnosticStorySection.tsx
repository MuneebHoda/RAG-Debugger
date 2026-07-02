import { ArrowRight, CheckCircle2, CircleX, TriangleAlert } from "lucide-react";
import { AnimatePresence, m, useReducedMotion } from "motion/react";
import { useState } from "react";

import { diagnosticStages, type DiagnosticStageId } from "./landingStoryData";
import { LandingStoryHeading } from "./LandingStoryHeading";
import { quickTransition } from "./motion";
import { useRovingTabs } from "./useRovingTabs";
import styles from "./DiagnosticStorySection.module.css";

const diagnosticStageIds = diagnosticStages.map((stage) => stage.id);

export function DiagnosticProofBand() {
  return (
    <section
      className={styles.proof}
      data-landing-header-tone="light"
      id="diagnostic-proof"
      aria-labelledby="diagnostic-proof-title"
    >
      <div className={styles.proofCopy}>
        <p>Example run · account recovery</p>
        <h2 id="diagnostic-proof-title">
          Retrieval found candidates. CorpusLab still refused to answer.
        </h2>
      </div>
      <dl className={styles.proofMetrics}>
        <div>
          <dt>Candidates retrieved</dt>
          <dd>3</dd>
        </div>
        <div>
          <dt>Supported evidence</dt>
          <dd>0</dd>
        </div>
        <div>
          <dt>Answerability</dt>
          <dd>Failed</dd>
        </div>
      </dl>
    </section>
  );
}

export function DiagnosticStorySection() {
  const [activeId, setActiveId] =
    useState<DiagnosticStageId>("unsupported_answer");
  const reducedMotion = useReducedMotion();
  const { handleTabKeyDown, registerTab } = useRovingTabs({
    ids: diagnosticStageIds,
    onSelect: setActiveId,
  });
  const activeStage =
    diagnosticStages.find((stage) => stage.id === activeId) ??
    diagnosticStages[diagnosticStages.length - 1];

  return (
    <section
      className={styles.story}
      data-landing-header-tone="dark"
      id="diagnostic-story"
      aria-labelledby="diagnostic-story-title"
    >
      <div className={styles.storyInner}>
        <LandingStoryHeading
          description="Inspect what ranked, what actually supports the answer, which label applies, and what to repair next."
          eyebrow="Failure to diagnosis"
          title="Find the exact evidence failure."
          titleId="diagnostic-story-title"
          tone="dark"
        />

        <div
          aria-label="RAG diagnostic stages"
          className={styles.stageTabs}
          role="tablist"
        >
          {diagnosticStages.map((stage, index) => (
            <button
              aria-controls="diagnostic-stage-panel"
              aria-selected={stage.id === activeStage.id}
              className={
                stage.id === activeStage.id
                  ? styles.activeStageTab
                  : styles.stageTab
              }
              id={`diagnostic-tab-${stage.id}`}
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

        <AnimatePresence initial={false} mode="wait">
          <m.div
            animate={{ opacity: 1, y: 0 }}
            aria-labelledby={`diagnostic-tab-${activeStage.id}`}
            className={styles.diagnosticPanel}
            exit={{ opacity: 0, y: reducedMotion ? 0 : -8 }}
            id="diagnostic-stage-panel"
            initial={{ opacity: 0, y: reducedMotion ? 0 : 8 }}
            key={activeStage.id}
            role="tabpanel"
            transition={reducedMotion ? { duration: 0 } : quickTransition}
          >
            <div className={styles.evidencePanel}>
              <div className={styles.panelHeading}>
                <span>Ranked evidence</span>
                <strong>{activeStage.evidence.length} candidates</strong>
              </div>
              <div className={styles.evidenceHeader} aria-hidden="true">
                <span>Rank</span>
                <span>Evidence</span>
                <span>Score</span>
              </div>
              <ol className={styles.evidenceList}>
                {activeStage.evidence.map((evidence, index) => (
                  <li key={evidence.label}>
                    <span className={styles.rank}>
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <div>
                      <strong>{evidence.label}</strong>
                      <span className={styles[evidence.support]}>
                        {supportIcon(evidence.support)}
                        {supportLabel(evidence.support)} · {evidence.signal}
                      </span>
                    </div>
                    <b>{evidence.score ?? "--"}</b>
                  </li>
                ))}
              </ol>
            </div>

            <div className={styles.diagnosis} aria-live="polite">
              <div className={styles.outcome}>
                <TriangleAlert aria-hidden="true" size={17} />
                <span>Outcome</span>
                <strong>{activeStage.outcome}</strong>
              </div>
              <div>
                <p>Primary diagnosis</p>
                <h3>{activeStage.title}</h3>
                <span>{activeStage.summary}</span>
              </div>
              <div className={styles.failureLabels}>
                <p>Failure labels</p>
                <div>
                  {activeStage.failureLabels.map((label) => (
                    <code key={label}>{label}</code>
                  ))}
                </div>
              </div>
              <div className={styles.recommendation}>
                <ArrowRight aria-hidden="true" size={17} />
                <span>
                  <small>Recommended repair</small>
                  <strong>{activeStage.recommendation}</strong>
                </span>
              </div>
            </div>
          </m.div>
        </AnimatePresence>
      </div>
    </section>
  );
}

function supportIcon(support: "supported" | "candidate_only" | "missing") {
  if (support === "supported") {
    return <CheckCircle2 aria-hidden="true" size={13} />;
  }
  if (support === "missing") return <CircleX aria-hidden="true" size={13} />;
  return <TriangleAlert aria-hidden="true" size={13} />;
}

function supportLabel(support: "supported" | "candidate_only" | "missing") {
  if (support === "supported") return "Supports answer";
  if (support === "missing") return "Expected evidence missing";
  return "Candidate only";
}
