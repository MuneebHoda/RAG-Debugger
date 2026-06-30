import {
  AlertTriangle,
  ArrowLeft,
  CheckCircle2,
  Gauge,
  XCircle,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";

import { getEvalLabExperiment } from "../../../lib/api/evalLab";
import { formatDateTime } from "../../../lib/dateTime";
import { CreateAuditReportAction } from "../reports/components/CreateAuditReportAction";
import styles from "./QualityPage.module.css";

export function ExperimentDetailPage() {
  const { experimentId } = useParams<{ experimentId: string }>();
  const experimentQuery = useQuery({
    queryKey: ["eval-experiment", experimentId],
    queryFn: ({ signal }) => getEvalLabExperiment(experimentId!, signal),
    enabled: Boolean(experimentId),
  });

  if (experimentQuery.isLoading) {
    return <div className={styles.empty}>Loading experiment result…</div>;
  }

  if (experimentQuery.isError || !experimentQuery.data) {
    return (
      <section className={styles.errorState} role="alert">
        <AlertTriangle aria-hidden="true" size={24} />
        <strong>This experiment could not be opened.</strong>
        <button type="button" onClick={() => void experimentQuery.refetch()}>
          Retry
        </button>
        <Link className={styles.secondaryButton} to="/app/evals">
          Back to Quality
        </Link>
      </section>
    );
  }

  const experiment = experimentQuery.data;
  const gatePassed = experiment.gate.status === "passed";
  return (
    <section className={styles.page} aria-labelledby="experiment-title">
      <Link
        className={styles.backLink}
        to={`/app/evals/datasets/${experiment.dataset_id}`}
      >
        <ArrowLeft aria-hidden="true" size={15} /> Back to dataset
      </Link>

      <header className={styles.header}>
        <div>
          <p>Experiment result</p>
          <h1 id="experiment-title">{experiment.name}</h1>
          <span>
            {experiment.dataset_name} · {formatDateTime(experiment.created_at)}
          </span>
        </div>
      </header>

      <CreateAuditReportAction
        source={{ sourceType: "experiment", sourceId: experiment.id }}
      />

      <section className={`${styles.gate} ${styles[experiment.gate.status]}`}>
        <div className={styles.gateIcon}>
          {gatePassed ? (
            <CheckCircle2 aria-hidden="true" size={20} />
          ) : (
            <XCircle aria-hidden="true" size={20} />
          )}
        </div>
        <div>
          <h2>Gate {experiment.gate.status}</h2>
          <p>{experiment.gate.reasons.join(" ")}</p>
        </div>
      </section>

      {!gatePassed ? (
        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div>
              <h2>Failed cases</h2>
              <p>Start here. These failures explain what needs attention.</p>
            </div>
            <span className={styles.failed}>{experiment.failures.length}</span>
          </div>
          <div className={styles.list}>
            {experiment.failures.map((failure, index) => (
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
          </div>
        </section>
      ) : null}

      <section className={styles.panel}>
        <div className={styles.panelHeading}>
          <div>
            <h2>Mode comparison</h2>
            <p>{experiment.comparison.summary}</p>
          </div>
          <Gauge aria-hidden="true" size={18} />
        </div>
        <div className={styles.modeResults}>
          {experiment.mode_results.map((result) => (
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
                  label="Passed cases"
                  value={`${result.passed_count}/${result.case_count}`}
                />
                <Metric
                  label="Latency p95"
                  value={`${result.latency_p95_ms} ms`}
                />
              </div>
            </article>
          ))}
        </div>
        <details className={styles.details}>
          <summary>Show gate thresholds and configuration</summary>
          <div className={styles.metricRows}>
            <Metric
              label="Recall threshold"
              value={percentage(experiment.gate.recall_threshold)}
            />
            <Metric
              label="Weak evidence limit"
              value={percentage(experiment.gate.weak_evidence_limit)}
            />
            <Metric label="Top k" value={String(experiment.top_k)} />
            <Metric
              label="Embedding model"
              value={experiment.config_snapshot.embedding_model.model_name}
            />
          </div>
        </details>
      </section>
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <span>
      {label} <strong>{value}</strong>
    </span>
  );
}

function percentage(value: number) {
  return `${Math.round(value * 100)}%`;
}
