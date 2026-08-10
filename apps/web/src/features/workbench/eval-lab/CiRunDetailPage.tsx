import { AlertTriangle, CheckCircle2, XCircle } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";

import { WorkbenchMetricCard } from "../../../components/workbench/WorkbenchMetricCard";
import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import { WorkbenchPanel } from "../../../components/workbench/WorkbenchPanel";
import { WorkbenchStatusPill } from "../../../components/workbench/WorkbenchStatusPill";
import {
  getCiEvalRun,
  type RetrievalEvalCaseRegression,
} from "../../../lib/api/evalLab";
import { formatDateTime } from "../../../lib/dateTime";
import { CreateAuditReportAction } from "../reports/components/CreateAuditReportAction";
import styles from "./QualityPage.module.css";

export function CiRunDetailPage() {
  const { runId = "" } = useParams();
  const runQuery = useQuery({
    queryKey: ["ci-eval-run", runId],
    queryFn: ({ signal }) => getCiEvalRun(runId, signal),
    enabled: Boolean(runId),
  });

  if (runQuery.isLoading) {
    return <div className={styles.empty}>Loading CI run…</div>;
  }

  if (runQuery.isError || !runQuery.data) {
    return (
      <section className={styles.errorState} role="alert">
        <AlertTriangle aria-hidden="true" size={24} />
        <strong>This CI run could not be opened.</strong>
        <button type="button" onClick={() => void runQuery.refetch()}>
          Retry
        </button>
        <Link className="secondary-button" to="/app/evals?view=ci-runs">
          Back to CI Runs
        </Link>
      </section>
    );
  }

  const run = runQuery.data;
  const gate = run.report.gate;
  const failed = run.gate_status === "failed";
  const regression = run.eval_regression;
  const regressedMetrics =
    regression?.metric_deltas.filter(
      (metric) => metric.classification === "regressed",
    ) ?? [];

  return (
    <section className={styles.page} aria-labelledby="ci-run-title">
      <WorkbenchPageHeader
        actions={
          failed ? (
            <CreateAuditReportAction
              compact
              source={{ sourceType: "ci_run", sourceId: run.id }}
            />
          ) : undefined
        }
        back={{ label: "Back to CI Runs", to: "/app/evals?view=ci-runs" }}
        description="Inspect the release gate, regression evidence, and failed cases recorded by automation."
        metadata={
          <>
            <span>{formatDateTime(run.created_at)}</span>
            <span>Config {run.config_label}</span>
          </>
        }
        section="CI run"
        title={run.dataset_name}
        titleId="ci-run-title"
      />

      <section className={styles.gate}>
        <div className={styles.gateIcon}>
          {failed ? (
            <XCircle aria-hidden="true" size={20} />
          ) : (
            <CheckCircle2 aria-hidden="true" size={20} />
          )}
        </div>
        <div>
          <h2>Gate {run.gate_status}</h2>
          <p>{gate.reasons.join(" ")}</p>
        </div>
      </section>

      <section className={styles.stats} aria-label="Gate thresholds">
        <WorkbenchMetricCard
          label="Recall@k"
          tone={
            gate.average_recall_at_k < gate.recall_threshold
              ? "danger"
              : "success"
          }
          value={percentage(gate.average_recall_at_k)}
        />
        <WorkbenchMetricCard
          label="Recall threshold"
          value={percentage(gate.recall_threshold)}
        />
        <WorkbenchMetricCard
          label="Weak evidence"
          tone={
            gate.weak_evidence_rate > gate.weak_evidence_limit
              ? "danger"
              : "neutral"
          }
          value={percentage(gate.weak_evidence_rate)}
        />
        <WorkbenchMetricCard
          label="Critical failures"
          tone={gate.critical_failure_count > 0 ? "danger" : "neutral"}
          value={String(gate.critical_failure_count)}
        />
      </section>

      <WorkbenchPanel
        className={styles.panel}
        description="The source revision and Eval Lab configuration saved with this run."
        title="Run metadata"
      >
        <dl className={`${styles.formGrid} ${styles.metadataList}`}>
          <Metadata label="Branch" value={run.branch ?? "Not provided"} />
          <Metadata
            label="Commit SHA"
            value={run.commit_sha ?? "Not provided"}
          />
          <Metadata label="Base ref" value={run.base_ref ?? "Not provided"} />
          <Metadata label="Head ref" value={run.head_ref ?? "Not provided"} />
          <Metadata label="Config label" value={run.config_label} />
          <Metadata label="Dataset" value={run.dataset_name} />
        </dl>
        <Link
          className="secondary-button"
          to={`/app/evals/experiments/${run.experiment_id}`}
        >
          Open Eval Lab experiment
        </Link>
      </WorkbenchPanel>

      {failed ? (
        <WorkbenchPanel
          className={styles.panel}
          description="Threshold failures and metrics that regressed from the previous compatible CI configuration."
          title="Failed metrics"
        >
          <ul className={styles.list}>
            {gate.reasons.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
            {regressedMetrics.map((metric) => (
              <li key={metric.metric}>
                {metricLabel(metric.metric)} changed from{" "}
                {metricValue(metric.metric, metric.baseline)} to{" "}
                {metricValue(metric.metric, metric.current)}.
              </li>
            ))}
          </ul>
        </WorkbenchPanel>
      ) : null}

      <WorkbenchPanel
        className={styles.panel}
        description="Eval Lab v2 compares this run only when the previous dataset/config run has the same top-k and mode set."
        title="Regression summary"
      >
        {regression ? (
          <>
            <WorkbenchStatusPill
              tone={
                regression.classification === "regressed"
                  ? "danger"
                  : regression.classification === "improved"
                    ? "success"
                    : "neutral"
              }
            >
              {regression.baseline_experiment_id
                ? regression.classification
                : "No baseline"}
            </WorkbenchStatusPill>
            <p>{regression.summary}</p>
            <div className={styles.formGrid}>
              <RegressionCases
                cases={regression.newly_failed_cases}
                empty="No newly failing cases."
                title="Newly failing cases"
              />
              <RegressionCases
                cases={regression.recovered_cases}
                empty="No recovered cases."
                title="Recovered cases"
              />
              <RegressionCases
                cases={regression.changed_top_evidence_cases}
                empty="No changed top evidence."
                title="Changed top evidence"
              />
              <RegressionCases
                cases={regression.changed_failure_label_cases}
                empty="No changed failure labels."
                title="Changed failure labels"
              />
            </div>
          </>
        ) : run.regression ? (
          <p>{run.regression.summary}</p>
        ) : (
          <p className={styles.empty}>
            No comparable CI baseline is available.
          </p>
        )}
      </WorkbenchPanel>

      {failed ? (
        <WorkbenchPanel
          className={styles.panel}
          description="Deterministic case failures and labels captured by the gate."
          title="Failed cases"
        >
          <div className={styles.list}>
            {run.report.failed_cases.map((failure, index) => (
              <article
                className={styles.failureCard}
                key={`${failure.case_id}-${failure.retrieval_mode}-${failure.label}-${index}`}
              >
                <strong>{failure.query}</strong>
                <p>{failure.message}</p>
                <small>
                  {failure.retrieval_mode} ·{" "}
                  {failure.label.replaceAll("_", " ")} · {failure.severity}
                </small>
              </article>
            ))}
            {run.report.failed_cases.length === 0 ? (
              <p className={styles.empty}>
                No case-level failures were recorded; review the failed metrics
                above.
              </p>
            ) : null}
          </div>
        </WorkbenchPanel>
      ) : null}

      <WorkbenchPanel
        className={styles.panel}
        description="Frozen per-mode metrics from the CI-triggered Eval Lab experiment."
        title="Metrics summary"
      >
        <div className={styles.modeResults}>
          {run.report.experiment.mode_results.map((result) => (
            <article className={styles.modeCard} key={result.retrieval_mode}>
              <h3>{result.retrieval_mode}</h3>
              <div className={styles.metricRows}>
                <Metric
                  label="Recall@k"
                  value={percentage(result.average_recall_at_k)}
                />
                <Metric
                  label="Precision@k"
                  value={percentage(result.average_precision_at_k)}
                />
                <Metric
                  label="MRR"
                  value={result.mean_reciprocal_rank.toFixed(2)}
                />
                <Metric
                  label="Citation coverage"
                  value={percentage(result.citation_coverage)}
                />
                <Metric
                  label="Weak evidence"
                  value={String(result.weak_evidence_count)}
                />
                <Metric
                  label="Latency p95"
                  value={`${result.latency_p95_ms} ms`}
                />
              </div>
            </article>
          ))}
        </div>
      </WorkbenchPanel>
    </section>
  );
}

function Metadata({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function RegressionCases({
  cases,
  empty,
  title,
}: {
  cases: RetrievalEvalCaseRegression[];
  empty: string;
  title: string;
}) {
  return (
    <section className={styles.list}>
      <h3>{title}</h3>
      {cases.map((entry) => (
        <article
          className={styles.failureCard}
          key={`${entry.case_id}-${entry.retrieval_mode}`}
        >
          <strong>{entry.query}</strong>
          <small>
            {entry.retrieval_mode} ·{" "}
            {entry.current_failure_labels
              .map((label) => label.replaceAll("_", " "))
              .join(", ") || "recovered"}
          </small>
        </article>
      ))}
      {cases.length === 0 ? <p className={styles.empty}>{empty}</p> : null}
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <span>
      {label}
      <strong>{value}</strong>
    </span>
  );
}

function percentage(value: number) {
  return `${Math.round(value * 100)}%`;
}

function metricLabel(metric: string) {
  return metric.replaceAll("_", " ");
}

function metricValue(metric: string, value: number | null) {
  if (value === null) return "no baseline";
  if (metric === "latency_p95_ms") return `${Math.round(value)} ms`;
  if (metric === "missing_embedding_failures") return String(Math.round(value));
  return percentage(value);
}
