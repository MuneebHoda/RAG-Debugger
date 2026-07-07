import { GitBranch } from "lucide-react";

import { WorkbenchEmptyState } from "../../../../components/workbench/WorkbenchEmptyState";
import { WorkbenchMetricCard } from "../../../../components/workbench/WorkbenchMetricCard";
import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import { WorkbenchStatusPill } from "../../../../components/workbench/WorkbenchStatusPill";
import type { CiEvalRun } from "../../../../lib/api/evalLab";
import styles from "../QualityPage.module.css";

export function CreateDatasetPanel({
  description,
  isPending,
  name,
  onCreate,
  onDescriptionChange,
  onNameChange,
}: {
  description: string;
  isPending: boolean;
  name: string;
  onCreate: () => void;
  onDescriptionChange: (value: string) => void;
  onNameChange: (value: string) => void;
}) {
  return (
    <WorkbenchPanel
      className={styles.panel}
      description="Group questions that must keep retrieving the right evidence."
      title="Create an eval dataset"
    >
      <div className={styles.form}>
        <div className={styles.formGrid}>
          <label>
            Dataset name
            <input
              value={name}
              onChange={(event) => onNameChange(event.currentTarget.value)}
            />
          </label>
          <label>
            Description
            <input
              value={description}
              onChange={(event) =>
                onDescriptionChange(event.currentTarget.value)
              }
            />
          </label>
        </div>
        <button
          className={styles.primaryButton}
          disabled={!name.trim() || isPending}
          type="button"
          onClick={onCreate}
        >
          Create dataset
        </button>
      </div>
    </WorkbenchPanel>
  );
}

export function CiRunsView({
  isLoading,
  runs,
}: {
  isLoading: boolean;
  runs: CiEvalRun[];
}) {
  const passed = runs.filter((run) => run.gate_status === "passed").length;
  const failed = runs.filter((run) => run.gate_status === "failed").length;

  return (
    <>
      <section className={styles.stats} aria-label="CI runs summary">
        <WorkbenchMetricCard label="Runs" value={String(runs.length)} />
        <WorkbenchMetricCard
          label="Passed"
          tone="success"
          value={String(passed)}
        />
        <WorkbenchMetricCard
          label="Failed"
          tone={failed > 0 ? "danger" : "neutral"}
          value={String(failed)}
        />
        <WorkbenchMetricCard
          label="Latest gate"
          tone={gateTone(runs[0]?.gate_status)}
          value={runs[0]?.gate_status ?? "Not run"}
        />
      </section>
      <WorkbenchPanel
        className={styles.panel}
        description="Dataset checks submitted by branches, commits, and CI jobs."
        icon={GitBranch}
        title="Automated quality gates"
      >
        <div className={styles.list}>
          {isLoading ? <p className={styles.empty}>Loading CI runs…</p> : null}
          {runs.map((run) => (
            <article className={styles.experimentCard} key={run.id}>
              <div className={styles.cardHeader}>
                <strong>{run.dataset_name}</strong>
                <WorkbenchStatusPill tone={gateTone(run.gate_status)}>
                  {run.gate_status}
                </WorkbenchStatusPill>
              </div>
              <p>
                {run.branch ?? "manual"} ·{" "}
                {run.commit_sha?.slice(0, 8) ?? "no commit"}
              </p>
            </article>
          ))}
          {!isLoading && runs.length === 0 ? (
            <WorkbenchEmptyState
              description="Create a workspace API key, then run an Eval Lab dataset from your CI workflow."
              icon={GitBranch}
              primaryAction={{
                label: "Manage API keys",
                to: "/app/settings?tab=api-keys",
              }}
              secondaryAction={{ label: "Open Eval Lab", to: "/app/evals" }}
              title="No CI quality runs"
            />
          ) : null}
        </div>
      </WorkbenchPanel>
    </>
  );
}

function gateTone(
  status: string | null | undefined,
): "success" | "danger" | "neutral" {
  if (status === "passed") return "success";
  if (status === "failed") return "danger";
  return "neutral";
}
