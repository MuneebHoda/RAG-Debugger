import { GitBranch } from "lucide-react";

import { WorkbenchEmptyState } from "../../../../components/workbench/WorkbenchEmptyState";
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
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Create an eval dataset</h2>
          <p>Group questions that must keep retrieving the right evidence.</p>
        </div>
      </div>
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
    </section>
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
        <QualityStat label="Runs" value={String(runs.length)} />
        <QualityStat label="Passed" value={String(passed)} />
        <QualityStat label="Failed" value={String(failed)} />
        <QualityStat
          label="Latest gate"
          value={runs[0]?.gate_status ?? "Not run"}
        />
      </section>
      <section className={styles.panel}>
        <div className={styles.panelHeading}>
          <div>
            <h2>Automated quality gates</h2>
            <p>Dataset checks submitted by branches, commits, and CI jobs.</p>
          </div>
          <GitBranch aria-hidden="true" size={18} />
        </div>
        <div className={styles.list}>
          {isLoading ? <p className={styles.empty}>Loading CI runs…</p> : null}
          {runs.map((run) => (
            <article className={styles.experimentCard} key={run.id}>
              <div className={styles.cardHeader}>
                <strong>{run.dataset_name}</strong>
                <span className={styles[run.gate_status]}>
                  {run.gate_status}
                </span>
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
      </section>
    </>
  );
}

function QualityStat({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.stat}>
      <small>{label}</small>
      <strong>{value}</strong>
    </div>
  );
}
