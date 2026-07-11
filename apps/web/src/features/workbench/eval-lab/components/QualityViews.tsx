import { GitBranch } from "lucide-react";
import { Link } from "react-router-dom";

import { WorkbenchEmptyState } from "../../../../components/workbench/WorkbenchEmptyState";
import { WorkbenchMetricCard } from "../../../../components/workbench/WorkbenchMetricCard";
import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import { WorkbenchStatusPill } from "../../../../components/workbench/WorkbenchStatusPill";
import type {
  CiEvalRun,
  RetrievalEvalExperiment,
  RetrievalEvalExperimentSummary,
  RetrievalEvalRegressionClassification,
  RetrievalEvalRegressionComparison,
  RetrievalEvalRegressionMetric,
  RetrievalEvalTrendSummary,
} from "../../../../lib/api/evalLab";
import { formatDateTime } from "../../../../lib/dateTime";
import { classifyBaselineCompatibility } from "../evalRegression";
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

export function TrendSummaryPanel({
  trend,
}: {
  trend: RetrievalEvalTrendSummary | null | undefined;
}) {
  const latest = trend?.points.at(-1);
  const regression = trend?.latest_regression;

  return (
    <WorkbenchPanel
      className={styles.panel}
      description="Track quality movement across recent Eval Lab experiments."
      title="Quality trend"
    >
      <div className={styles.stats}>
        <TrendMetric
          label="Latest gate"
          value={trend?.latest_gate_status ?? "Not run"}
        />
        <TrendMetric
          label="Recall@k"
          value={latest ? percentage(latest.average_recall_at_k) : "—"}
        />
        <TrendMetric
          label="Precision@k"
          value={latest ? percentage(latest.average_precision_at_k) : "—"}
        />
        <TrendMetric
          label="Regression"
          value={regression?.classification ?? "No baseline"}
        />
      </div>
      {regression ? (
        <div className={styles.details}>
          <WorkbenchStatusPill
            tone={classificationTone(regression.classification)}
          >
            {regression.classification}
          </WorkbenchStatusPill>
          <p>{regression.summary}</p>
          <small>
            Newly failed {regression.newly_failed_cases.length} · recovered{" "}
            {regression.recovered_cases.length}
          </small>
        </div>
      ) : (
        <p className={styles.empty}>
          Run at least two comparable experiments to see trend movement.
        </p>
      )}
    </WorkbenchPanel>
  );
}

export function ExperimentHistoryPanel({
  experiments,
  isLoading,
}: {
  experiments: RetrievalEvalExperimentSummary[];
  isLoading: boolean;
}) {
  return (
    <WorkbenchPanel
      className={styles.panel}
      description="Recent runs for this dataset, ordered newest first."
      title="Experiment history"
    >
      <div className={styles.list}>
        {isLoading ? (
          <p className={styles.empty}>Loading experiment history…</p>
        ) : null}
        {experiments.map((experiment) => (
          <Link
            className={styles.experimentCard}
            key={experiment.id}
            to={`/app/evals/experiments/${experiment.id}`}
          >
            <div className={styles.cardHeader}>
              <strong>{experiment.name}</strong>
              <WorkbenchStatusPill tone={gateTone(experiment.gate_status)}>
                {experiment.gate_status}
              </WorkbenchStatusPill>
            </div>
            <p>
              {formatDateTime(experiment.created_at)} · best{" "}
              {experiment.best_mode ?? "none"} · R{" "}
              {percentage(experiment.average_recall_at_k)} · P{" "}
              {percentage(experiment.average_precision_at_k)}
            </p>
          </Link>
        ))}
        {!isLoading && experiments.length === 0 ? (
          <WorkbenchEmptyState
            description="Run this dataset once to create the first quality baseline."
            icon={GitBranch}
            title="No experiment history"
          />
        ) : null}
      </div>
    </WorkbenchPanel>
  );
}

export function BaselineSelector({
  automaticBaseline,
  currentExperiment,
  error,
  experiments,
  isLoading,
  onBaselineChange,
  selectedBaselineId,
}: {
  automaticBaseline: RetrievalEvalExperimentSummary | null;
  currentExperiment: RetrievalEvalExperiment;
  error: string | null;
  experiments: RetrievalEvalExperimentSummary[];
  isLoading: boolean;
  onBaselineChange: (baselineId: string | null) => void;
  selectedBaselineId: string | null;
}) {
  const selectedCandidate =
    experiments.find((candidate) => candidate.id === selectedBaselineId) ??
    null;
  const selectedCompatibility = selectedCandidate
    ? classifyBaselineCompatibility(selectedCandidate, currentExperiment)
    : null;

  return (
    <WorkbenchPanel
      className={styles.panel}
      description="Choose which earlier experiment this result should be compared against."
      title="Comparison baseline"
    >
      <div className={styles.form}>
        <label>
          Baseline experiment
          <select
            aria-label="Baseline experiment"
            disabled={isLoading}
            value={selectedBaselineId ?? "auto"}
            onChange={(event) =>
              onBaselineChange(
                event.currentTarget.value === "auto"
                  ? null
                  : event.currentTarget.value,
              )
            }
          >
            <option value="auto">
              Automatic ·{" "}
              {automaticBaseline
                ? automaticBaseline.name
                : "latest compatible run"}
            </option>
            {experiments.map((candidate) => {
              const compatibility = classifyBaselineCompatibility(
                candidate,
                currentExperiment,
              );
              return (
                <option
                  disabled={compatibility.level === "incompatible"}
                  key={candidate.id}
                  value={candidate.id}
                >
                  {candidate.name} · {compatibility.label}
                </option>
              );
            })}
          </select>
        </label>
        <div className={styles.baselineSummary}>
          <strong>
            {selectedCandidate
              ? selectedCandidate.name
              : automaticBaseline
                ? `Automatic: ${automaticBaseline.name}`
                : "No automatic baseline"}
          </strong>
          <span>
            {selectedCompatibility?.reason ??
              (automaticBaseline
                ? "Using the latest earlier experiment with the same top_k and retrieval modes."
                : "Run another compatible experiment to enable regression comparison.")}
          </span>
          {selectedCandidate ? (
            <small>
              {formatDateTime(selectedCandidate.created_at)} · gate{" "}
              {selectedCandidate.gate_status} · modes{" "}
              {selectedCandidate.modes.join(", ")} · top_k{" "}
              {selectedCandidate.top_k}
            </small>
          ) : null}
        </div>
      </div>
      {selectedCompatibility?.level === "partially_compatible" ? (
        <p className={styles.warning}>
          Partial baseline: compare directionally, because top_k or mode
          coverage differs.
        </p>
      ) : null}
      {error ? <p className={styles.error}>{error}</p> : null}
    </WorkbenchPanel>
  );
}

export function RegressionPanel({
  baselineExperiment,
  currentExperiment,
  regression,
}: {
  baselineExperiment: RetrievalEvalExperimentSummary | null;
  currentExperiment: RetrievalEvalExperimentSummary | null;
  regression: RetrievalEvalRegressionComparison | null | undefined;
}) {
  if (!regression) {
    return null;
  }

  const hasBaseline = Boolean(regression.baseline_experiment_id);
  const displayClassification = hasBaseline
    ? regression.classification
    : "No comparable baseline";
  const explanation = regressionExplanation(regression);

  return (
    <section className={styles.panel} aria-labelledby="regression-title">
      <div className={styles.panelHeading}>
        <div>
          <h2 id="regression-title">Regression history</h2>
          <p>{hasBaseline ? regression.summary : explanation}</p>
        </div>
        <WorkbenchStatusPill
          tone={
            hasBaseline
              ? classificationTone(regression.classification)
              : "neutral"
          }
        >
          {displayClassification}
        </WorkbenchStatusPill>
      </div>
      <div className={styles.grid}>
        <ExperimentIdentity
          experiment={baselineExperiment}
          label="Baseline"
          status={regression.baseline_gate_status}
        />
        <ExperimentIdentity
          experiment={currentExperiment}
          label="Current"
          status={regression.current_gate_status}
        />
      </div>
      {hasBaseline ? (
        <p className={styles.callout}>{explanation}</p>
      ) : (
        <p className={styles.callout}>
          This experiment is the first compatible run for its dataset, top_k,
          and retrieval modes.
        </p>
      )}
      <div className={styles.metricRows}>
        {regression.metric_deltas.map((delta) => (
          <span key={delta.metric}>
            {delta.metric.replaceAll("_", " ")}
            <strong>
              {metricValue(delta.metric, delta.current)}{" "}
              {delta.baseline === null
                ? "No baseline"
                : signedDelta(delta.delta)}
            </strong>
          </span>
        ))}
      </div>
      <div className={styles.grid}>
        <CaseRegressionList
          cases={regression.newly_failed_cases}
          kind="status"
          title="Newly failed"
        />
        <CaseRegressionList
          cases={regression.recovered_cases}
          kind="status"
          title="Recovered"
        />
        <CaseRegressionList
          cases={regression.changed_top_evidence_cases}
          kind="evidence"
          title="Changed top evidence"
        />
        <CaseRegressionList
          cases={regression.changed_failure_label_cases}
          kind="labels"
          title="Changed failure labels"
        />
      </div>
    </section>
  );
}

function ExperimentIdentity({
  experiment,
  label,
  status,
}: {
  experiment: RetrievalEvalExperimentSummary | null;
  label: string;
  status: string | null;
}) {
  return (
    <article className={styles.identityCard}>
      <small>{label}</small>
      <strong>{experiment?.name ?? "No comparable baseline"}</strong>
      <span>
        {experiment ? formatDateTime(experiment.created_at) : "Not available"} ·
        gate {status ?? "none"}
      </span>
      {experiment ? (
        <span>
          modes {experiment.modes.join(", ")} · top_k {experiment.top_k}
        </span>
      ) : null}
    </article>
  );
}

function CaseRegressionList({
  cases,
  kind,
  title,
}: {
  cases: RetrievalEvalRegressionComparison["newly_failed_cases"];
  kind: "status" | "evidence" | "labels";
  title: string;
}) {
  return (
    <div className={styles.list}>
      <h3>{title}</h3>
      {cases.slice(0, 5).map((entry) => (
        <article
          className={styles.failureCard}
          key={`${title}-${entry.case_id}-${entry.retrieval_mode}`}
        >
          <strong>{entry.query}</strong>
          <BeforeAfter entry={entry} kind={kind} />
        </article>
      ))}
      {cases.length === 0 ? (
        <p className={styles.empty}>None detected.</p>
      ) : null}
    </div>
  );
}

function BeforeAfter({
  entry,
  kind,
}: {
  entry: RetrievalEvalRegressionComparison["newly_failed_cases"][number];
  kind: "status" | "evidence" | "labels";
}) {
  if (kind === "evidence") {
    return (
      <div className={styles.metricRows}>
        <span>
          Before{" "}
          <strong>{compactIds(entry.baseline_retrieved_chunk_ids)}</strong>
        </span>
        <span>
          After <strong>{compactIds(entry.current_retrieved_chunk_ids)}</strong>
        </span>
      </div>
    );
  }

  if (kind === "labels") {
    return (
      <div className={styles.metricRows}>
        <span>
          Before <strong>{formatLabels(entry.baseline_failure_labels)}</strong>
        </span>
        <span>
          After <strong>{formatLabels(entry.current_failure_labels)}</strong>
        </span>
      </div>
    );
  }

  return (
    <small>
      {entry.retrieval_mode} · rank {entry.baseline_top_hit_rank ?? "—"} →{" "}
      {entry.current_top_hit_rank ?? "—"} ·{" "}
      {entry.baseline_passed === null
        ? "no baseline"
        : entry.baseline_passed
          ? "passed"
          : "failed"}{" "}
      →{" "}
      {entry.current_passed === null
        ? "not run"
        : entry.current_passed
          ? "passed"
          : "failed"}
    </small>
  );
}

function TrendMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.stat}>
      <small>{label}</small>
      <strong>{value}</strong>
    </div>
  );
}

function classificationTone(
  classification: RetrievalEvalRegressionClassification | null | undefined,
): "success" | "danger" | "neutral" {
  if (classification === "improved") return "success";
  if (classification === "regressed") return "danger";
  return "neutral";
}

function metricValue(metric: string, value: number) {
  if (metric === "latency_p95_ms") return `${Math.round(value)} ms`;
  if (metric === "missing_embedding_failures") return String(Math.round(value));
  return percentage(value);
}

function regressionExplanation(regression: RetrievalEvalRegressionComparison) {
  if (!regression.baseline_experiment_id) {
    return "No comparable baseline exists yet for this experiment.";
  }

  if (
    regression.baseline_gate_status === "passed" &&
    regression.current_gate_status === "failed"
  ) {
    return "Classification is regressed because the release gate moved from passed to failed.";
  }

  if (
    regression.baseline_gate_status === "failed" &&
    regression.current_gate_status === "passed"
  ) {
    return "Classification is improved because the release gate moved from failed to passed.";
  }

  if (regression.newly_failed_cases.length > 0) {
    return `Classification is regressed because ${regression.newly_failed_cases.length} case${plural(regression.newly_failed_cases.length)} newly failed.`;
  }

  if (regression.recovered_cases.length > 0) {
    return `Classification is improved because ${regression.recovered_cases.length} case${plural(regression.recovered_cases.length)} recovered.`;
  }

  const decisiveDelta = regression.metric_deltas.find(
    (delta) => delta.classification !== "unchanged",
  );
  if (decisiveDelta) {
    return `Classification is ${decisiveDelta.classification} because ${metricLabel(decisiveDelta.metric)} changed from ${metricValue(decisiveDelta.metric, decisiveDelta.baseline ?? 0)} to ${metricValue(decisiveDelta.metric, decisiveDelta.current)}.`;
  }

  return "Classification is unchanged because gate status, cases, and metric deltas stayed within thresholds.";
}

function compactIds(ids: string[]) {
  if (ids.length === 0) return "none";
  return ids.map((id) => id.slice(0, 8)).join(", ");
}

function formatLabels(labels: string[]) {
  if (labels.length === 0) return "none";
  return labels.map((label) => label.replaceAll("_", " ")).join(", ");
}

function metricLabel(metric: RetrievalEvalRegressionMetric) {
  return metric.replaceAll("_", " ");
}

function plural(count: number) {
  return count === 1 ? "" : "s";
}

function percentage(value: number) {
  return `${Math.round(value * 100)}%`;
}

function signedDelta(value: number) {
  if (Math.abs(value) < 0.005) return "0";
  return value > 0 ? `+${formatDelta(value)}` : formatDelta(value);
}

function formatDelta(value: number) {
  if (Math.abs(value) > 2) return `${Math.round(value)}`;
  return `${Math.round(value * 100)} pts`;
}

function gateTone(
  status: string | null | undefined,
): "success" | "danger" | "neutral" {
  if (status === "passed") return "success";
  if (status === "failed") return "danger";
  return "neutral";
}
