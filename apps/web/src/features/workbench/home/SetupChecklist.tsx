import { ArrowRight, Check, CircleCheck, Loader2 } from "lucide-react";
import { Link } from "react-router-dom";

import type { DemoStatus } from "../../../lib/api/demo";
import styles from "./SetupChecklist.module.css";

interface SetupStep {
  id: string;
  title: string;
  complete: boolean;
}

export function SetupChecklist({
  error,
  isLoading,
  isMutating,
  status,
  onIndex,
  onLoad,
  onRetry,
}: {
  error: Error | null;
  isLoading: boolean;
  isMutating: boolean;
  status?: DemoStatus;
  onIndex: () => void;
  onLoad: () => void;
  onRetry: () => void;
}) {
  if (isLoading) {
    return (
      <section className={styles.card}>Loading guided demo status…</section>
    );
  }
  if (error || !status) {
    return (
      <section className={styles.card} role="alert">
        <strong>Guided demo status is unavailable.</strong>
        <button className={styles.actionButton} type="button" onClick={onRetry}>
          Retry
        </button>
      </section>
    );
  }

  const steps = buildSteps(status);
  const completedCount = steps.filter((step) => step.complete).length;
  const currentStep = steps.find((step) => !step.complete) ?? null;

  if (!currentStep) {
    return (
      <section className={styles.completeCard} aria-label="Setup complete">
        <span>
          <CircleCheck aria-hidden="true" size={18} /> Workflow ready
        </span>
        <Link to={`/app/reports/${status.progress.report_id}`}>
          Open completed audit report
        </Link>
      </section>
    );
  }

  return (
    <section className={styles.card} aria-labelledby="setup-title">
      <div className={styles.header}>
        <div>
          <p>Five-minute guided demo</p>
          <h2 id="setup-title">From sample corpus to audit report</h2>
        </div>
        <span className={styles.progress}>{completedCount}/6 complete</span>
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

      <NextAction
        currentStep={currentStep.id}
        isMutating={isMutating}
        status={status}
        onIndex={onIndex}
        onLoad={onLoad}
      />
    </section>
  );
}

function buildSteps(status: DemoStatus): SetupStep[] {
  const progress = status.progress;
  return [
    {
      id: "load",
      title: "Sample corpus loaded",
      complete: progress.sample_corpus_loaded,
    },
    {
      id: "chunk",
      title: "Chunks created",
      complete: progress.chunks_created,
    },
    {
      id: "embed",
      title: "Embeddings indexed",
      complete: progress.embeddings_indexed,
    },
    {
      id: "retrieve",
      title: "Retrieval query run",
      complete: progress.retrieval_run_id !== null,
    },
    {
      id: "trace",
      title: "Trace saved",
      complete: progress.trace_id !== null,
    },
    {
      id: "report",
      title: "Audit report generated",
      complete: progress.report_id !== null,
    },
  ];
}

function NextAction({
  currentStep,
  isMutating,
  status,
  onIndex,
  onLoad,
}: {
  currentStep: string;
  isMutating: boolean;
  status: DemoStatus;
  onIndex: () => void;
  onLoad: () => void;
}) {
  if (currentStep === "load" || currentStep === "chunk") {
    return (
      <div className={styles.actionRow}>
        <span>
          Load or repair three versioned sample documents without changing
          existing data.
        </span>
        <button
          className={styles.actionButton}
          disabled={isMutating}
          type="button"
          onClick={onLoad}
        >
          {isMutating ? <Loader2 className="spin" size={16} /> : null}
          {currentStep === "load"
            ? "Load sample corpus"
            : "Repair sample corpus"}
        </button>
      </div>
    );
  }
  if (currentStep === "embed") {
    return (
      <div className={styles.actionRow}>
        <span>
          Index only the sample source so hybrid retrieval can compare semantic
          evidence.
        </span>
        <button
          className={styles.actionButton}
          disabled={isMutating}
          type="button"
          onClick={onIndex}
        >
          {isMutating ? <Loader2 className="spin" size={16} /> : null} Index
          sample
        </button>
      </div>
    );
  }
  if (currentStep === "retrieve") {
    const recommended =
      status.suggested_queries.find((query) => query.recommended) ??
      status.suggested_queries[0];
    return (
      <div className={styles.actionRow}>
        <span>
          Run the recommended diagnostic question against only the sample
          corpus.
        </span>
        <Link to={`/app/retrieval?demo_query=${recommended.id}`}>
          Test recommended query <ArrowRight aria-hidden="true" size={16} />
        </Link>
      </div>
    );
  }
  if (currentStep === "trace") {
    return (
      <div className={styles.actionRow}>
        <span>
          Reopen the guided question, then use Debug this run to save its trace.
        </span>
        <Link to="/app/retrieval?demo_query=account_recovery">
          Run and debug <ArrowRight aria-hidden="true" size={16} />
        </Link>
      </div>
    );
  }
  return (
    <div className={styles.actionRow}>
      <span>Create a metadata-only audit report from the saved trace.</span>
      <Link to={`/app/traces/${status.progress.trace_id}`}>
        Open saved trace <ArrowRight aria-hidden="true" size={16} />
      </Link>
    </div>
  );
}
