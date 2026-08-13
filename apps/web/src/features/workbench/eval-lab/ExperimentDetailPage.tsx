import { AlertTriangle, CheckCircle2, Gauge, XCircle } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams, useSearchParams } from "react-router-dom";

import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import {
  getEvalLabExperiment,
  getEvalLabExperimentRegression,
  experimentContainsFullLocalData,
  listEvalLabDatasetExperiments,
} from "../../../lib/api/evalLab";
import { formatDateTime } from "../../../lib/dateTime";
import { CreateAuditReportAction } from "../reports/components/CreateAuditReportAction";
import { BaselineSelector, RegressionPanel } from "./components/QualityViews";
import { EvidenceStateList } from "./evidence/EvidenceStateList";
import {
  classifyBaselineCompatibility,
  findAutomaticBaseline,
  summarizeExperimentForComparison,
} from "./evalRegression";
import styles from "./QualityPage.module.css";

export function ExperimentDetailPage() {
  const { experimentId } = useParams<{ experimentId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedBaselineId = searchParams.get("baseline_id");
  const experimentQuery = useQuery({
    queryKey: ["eval-experiment", experimentId],
    queryFn: ({ signal }) => getEvalLabExperiment(experimentId!, signal),
    enabled: Boolean(experimentId),
  });
  const historyQuery = useQuery({
    queryKey: ["eval-dataset-experiments", experimentQuery.data?.dataset_id],
    queryFn: ({ signal }) =>
      listEvalLabDatasetExperiments(experimentQuery.data!.dataset_id, signal),
    enabled: Boolean(experimentQuery.data?.dataset_id),
  });

  const selectedBaseline = historyQuery.data?.find(
    (candidate) => candidate.id === selectedBaselineId,
  );
  const selectedCompatibility =
    selectedBaseline && experimentQuery.data
      ? classifyBaselineCompatibility(selectedBaseline, experimentQuery.data)
      : null;
  const hasInvalidSelectedBaseline = Boolean(
    selectedBaselineId &&
    historyQuery.isSuccess &&
    (!selectedBaseline || selectedCompatibility?.level === "incompatible"),
  );
  const hasSelectedBaselinePendingValidation = Boolean(
    selectedBaselineId && !historyQuery.isSuccess,
  );

  const regressionQuery = useQuery({
    queryKey: [
      "eval-experiment-regression",
      experimentId,
      selectedBaselineId ?? "auto",
    ],
    queryFn: ({ signal }) =>
      getEvalLabExperimentRegression(
        experimentId!,
        selectedBaselineId ?? undefined,
        signal,
      ),
    enabled:
      Boolean(experimentQuery.data) &&
      !hasInvalidSelectedBaseline &&
      !hasSelectedBaselinePendingValidation,
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
        <Link className="secondary-button" to="/app/evals">
          Back to Quality
        </Link>
      </section>
    );
  }

  const experiment = experimentQuery.data;
  const gatePassed = experiment.gate.status === "passed";
  const automaticBaseline = historyQuery.data
    ? findAutomaticBaseline(experiment, historyQuery.data)
    : null;
  const regressionBaseline =
    historyQuery.data?.find(
      (candidate) =>
        candidate.id === regressionQuery.data?.baseline_experiment_id,
    ) ?? null;
  const currentSummary =
    historyQuery.data?.find((candidate) => candidate.id === experiment.id) ??
    summarizeExperimentForComparison(experiment);
  const evidenceStateResult =
    experiment.mode_results.find(
      (result) => result.retrieval_mode === experiment.comparison.best_mode,
    ) ??
    experiment.mode_results[0] ??
    null;

  const updateBaseline = (baselineId: string | null) => {
    const nextParams = new URLSearchParams(searchParams);
    if (baselineId) {
      nextParams.set("baseline_id", baselineId);
    } else {
      nextParams.delete("baseline_id");
    }
    setSearchParams(nextParams, { replace: true });
  };

  return (
    <section className={styles.page} aria-labelledby="experiment-title">
      <WorkbenchPageHeader
        actions={
          <CreateAuditReportAction
            allowedPrivacyModes={
              experimentContainsFullLocalData(experiment) ? [] : undefined
            }
            compact
            disabledReason={
              experimentContainsFullLocalData(experiment)
                ? "Full-local imported Eval content cannot create or enter reports."
                : undefined
            }
            source={{ sourceType: "experiment", sourceId: experiment.id }}
          />
        }
        back={{
          label: "Back to dataset",
          to: `/app/evals/datasets/${experiment.dataset_id}`,
        }}
        description="Review gate outcome, failed cases, and retrieval-mode performance."
        metadata={
          <>
            <span>{experiment.dataset_name}</span>
            <span>{formatDateTime(experiment.created_at)}</span>
          </>
        }
        section="Eval experiment"
        title={experiment.name}
        titleId="experiment-title"
      />

      <section className={styles.gate}>
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

      <BaselineSelector
        automaticBaseline={automaticBaseline}
        currentExperiment={experiment}
        error={
          hasInvalidSelectedBaseline
            ? "Selected baseline cannot be compared with this experiment."
            : historyQuery.isError
              ? "Experiment history could not be loaded."
              : null
        }
        experiments={historyQuery.data ?? []}
        isLoading={historyQuery.isLoading}
        selectedBaselineId={selectedBaselineId}
        onBaselineChange={updateBaseline}
      />

      <RegressionPanel
        baselineExperiment={regressionBaseline}
        currentExperiment={currentSummary}
        error={
          hasInvalidSelectedBaseline
            ? "Choose Automatic or a compatible earlier experiment to view regression history."
            : regressionQuery.isError
              ? "Regression comparison could not be loaded."
              : null
        }
        regression={hasInvalidSelectedBaseline ? null : regressionQuery.data}
      />

      {!gatePassed ? (
        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div>
              <h2>Failed cases</h2>
              <p>Start here. These failures explain what needs attention.</p>
            </div>
            <span className={styles.error}>{experiment.failures.length}</span>
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

      {evidenceStateResult ? (
        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div>
              <h2>Evidence states</h2>
              <p>
                Expected and retrieved evidence states from the{" "}
                {evidenceStateResult.retrieval_mode} run.
              </p>
            </div>
          </div>
          <div className={styles.list}>
            {evidenceStateResult.case_results.slice(0, 5).map((result) => (
              <article className={styles.failureCard} key={result.case_id}>
                <strong>{result.query}</strong>
                <EvidenceStateList
                  context={{
                    kind: "retrieval",
                    hits: result.retrieved_chunk_ids.map((chunkId, index) => ({
                      chunkId,
                      rank: index + 1,
                    })),
                  }}
                  failureLabels={result.failures.map(
                    (failure) => failure.label,
                  )}
                  selection={{
                    documentIds: result.expected_document_ids,
                    chunkIds: result.expected_chunk_ids,
                  }}
                  title="Case evidence states"
                />
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
