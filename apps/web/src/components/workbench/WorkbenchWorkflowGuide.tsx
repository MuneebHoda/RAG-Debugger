import { ArrowRight, CircleDot, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router-dom";

import { WorkbenchPanel } from "./WorkbenchPanel";
import styles from "./WorkbenchWorkflowGuide.module.css";

export type WorkbenchWorkflowStepId =
  | "corpus"
  | "chunks"
  | "embeddings"
  | "retrieval"
  | "trace"
  | "quality"
  | "ci"
  | "report";

export interface WorkbenchWorkflowAction {
  disabled?: boolean;
  href?: string;
  label: string;
  onClick?: () => void;
  to?: string;
}

const WORKBENCH_QUALITY_LOOP_STEPS: Array<{
  id: WorkbenchWorkflowStepId;
  label: string;
}> = [
  { id: "corpus", label: "Corpus" },
  { id: "chunks", label: "Chunks" },
  { id: "embeddings", label: "Embeddings" },
  { id: "retrieval", label: "Retrieval" },
  { id: "trace", label: "Trace" },
  { id: "quality", label: "Eval" },
  { id: "ci", label: "CI gate" },
  { id: "report", label: "Report" },
];

export function WorkbenchWorkflowGuide({
  currentStep,
  icon,
  impact,
  nextAction,
  purpose,
  title = "Workflow guide",
}: {
  currentStep: WorkbenchWorkflowStepId;
  icon?: LucideIcon;
  impact: ReactNode;
  nextAction: WorkbenchWorkflowAction;
  purpose: ReactNode;
  title?: string;
}) {
  const currentStepLabel =
    WORKBENCH_QUALITY_LOOP_STEPS.find((step) => step.id === currentStep)
      ?.label ?? "Workflow";

  return (
    <WorkbenchPanel
      className={styles.guide}
      density="compact"
      icon={icon ?? CircleDot}
      title={title}
      tone="accent"
    >
      <div className={styles.content}>
        <div className={styles.copyGrid}>
          <div>
            <span>What this page is for</span>
            <p>{purpose}</p>
          </div>
          <div>
            <span>Why it improves retrieval</span>
            <p>{impact}</p>
          </div>
        </div>

        <div className={styles.loop} aria-label="RAG quality loop">
          {WORKBENCH_QUALITY_LOOP_STEPS.map((step) => (
            <span
              aria-current={step.id === currentStep ? "step" : undefined}
              className={step.id === currentStep ? styles.activeStep : ""}
              key={step.id}
            >
              {step.label}
            </span>
          ))}
        </div>

        <div className={styles.nextAction}>
          <span>
            You are at <strong>{currentStepLabel}</strong>. Recommended next
            action:
          </span>
          <WorkflowAction action={nextAction} />
        </div>
      </div>
    </WorkbenchPanel>
  );
}

function WorkflowAction({ action }: { action: WorkbenchWorkflowAction }) {
  const content = (
    <>
      {action.label} <ArrowRight aria-hidden="true" size={14} />
    </>
  );

  if (action.to) {
    return (
      <Link className="primary-button" to={action.to}>
        {content}
      </Link>
    );
  }

  if (action.href) {
    return (
      <a className="primary-button" href={action.href}>
        {content}
      </a>
    );
  }

  return (
    <button
      className="primary-button"
      disabled={action.disabled}
      type="button"
      onClick={action.onClick}
    >
      {content}
    </button>
  );
}
