import { ArrowRight, Check, CircleCheck } from "lucide-react";
import { Link } from "react-router-dom";

import type { OverviewResponse } from "../../../lib/api/overview";
import styles from "./SetupChecklist.module.css";

interface SetupStep {
  id: string;
  title: string;
  detail: string;
  action: string;
  route: string;
  complete: boolean;
}

export function SetupChecklist({ overview }: { overview: OverviewResponse }) {
  const steps = buildSteps(overview);
  const completedCount = steps.filter((step) => step.complete).length;
  const currentStep = steps.find((step) => !step.complete) ?? null;

  if (!currentStep) {
    return (
      <section className={styles.completeCard} aria-label="Setup complete">
        <span>
          <CircleCheck aria-hidden="true" size={18} /> Workflow ready
        </span>
        <Link to="/app/retrieval">Start another retrieval test</Link>
      </section>
    );
  }

  return (
    <section className={styles.card} aria-labelledby="setup-title">
      <div className={styles.header}>
        <div>
          <p>Guided setup</p>
          <h2 id="setup-title">Get to a trusted retrieval result</h2>
        </div>
        <span className={styles.progress}>{completedCount}/5 complete</span>
      </div>

      <ol className={styles.steps}>
        {steps.map((step, index) => (
          <li
            className={
              step.complete
                ? styles.complete
                : step.id === currentStep.id
                  ? styles.current
                  : styles.step
            }
            key={step.id}
          >
            <span className={styles.stepIcon}>
              {step.complete ? (
                <Check aria-hidden="true" size={14} />
              ) : (
                index + 1
              )}
            </span>
            <strong>{step.title}</strong>
          </li>
        ))}
      </ol>

      <div className={styles.actionRow}>
        <span>{currentStep.detail}</span>
        <Link to={currentStep.route}>
          {currentStep.action} <ArrowRight aria-hidden="true" size={16} />
        </Link>
      </div>
    </section>
  );
}

function buildSteps(overview: OverviewResponse): SetupStep[] {
  const pipeline = new Map(overview.pipeline.map((step) => [step.id, step]));
  const isComplete = (id: string) => pipeline.get(id)?.count !== 0;

  return [
    {
      id: "ingest",
      title: "Add documents",
      detail: "Add the documents CorpusLab should search.",
      action: "Open Corpus",
      route: "/app/sources",
      complete: isComplete("ingest"),
    },
    {
      id: "embed",
      title: "Index evidence",
      detail: "Index document chunks so semantic retrieval can use them.",
      action: "Index evidence",
      route: "/app/retrieval",
      complete:
        overview.embedding_status.total_chunks > 0 &&
        overview.embedding_status.missing_chunks === 0 &&
        overview.embedding_status.stale_chunks === 0,
    },
    {
      id: "retrieve",
      title: "Test retrieval",
      detail: "Ask one important question and inspect the cited evidence.",
      action: "Test a question",
      route: "/app/retrieval",
      complete: isComplete("retrieve"),
    },
    {
      id: "trace",
      title: "Save a run",
      detail:
        "Save the result so ranking decisions can be debugged and compared.",
      action: "Open Runs",
      route: "/app/traces",
      complete: isComplete("trace"),
    },
    {
      id: "eval",
      title: "Add a quality case",
      detail: "Record expected evidence to catch retrieval regressions.",
      action: "Open Quality",
      route: "/app/evals",
      complete: isComplete("eval"),
    },
  ];
}
