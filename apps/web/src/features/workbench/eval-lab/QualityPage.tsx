import {
  AlertTriangle,
  CheckCircle2,
  FlaskConical,
  Plus,
  XCircle,
} from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useSearchParams } from "react-router-dom";

import { WorkbenchEmptyState } from "../../../components/workbench/WorkbenchEmptyState";
import { WorkbenchMetricCard } from "../../../components/workbench/WorkbenchMetricCard";
import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import { WorkbenchPanel } from "../../../components/workbench/WorkbenchPanel";
import { WorkbenchStatusPill } from "../../../components/workbench/WorkbenchStatusPill";
import { WorkbenchWorkflowGuide } from "../../../components/workbench/WorkbenchWorkflowGuide";
import {
  createEvalLabDataset,
  listCiEvalRuns,
  listEvalLabDatasets,
  listEvalLabExperiments,
} from "../../../lib/api/evalLab";
import { formatDateTime } from "../../../lib/dateTime";
import { CiRunsView, CreateDatasetPanel } from "./components/QualityViews";
import styles from "./QualityPage.module.css";

export function QualityPage() {
  const [searchParams] = useSearchParams();
  const ciRunsView = searchParams.get("view") === "ci-runs";
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const queryClient = useQueryClient();
  const datasetsQuery = useQuery({
    queryKey: ["eval-datasets"],
    queryFn: ({ signal }) => listEvalLabDatasets(signal),
  });
  const experimentsQuery = useQuery({
    queryKey: ["eval-experiments"],
    queryFn: ({ signal }) => listEvalLabExperiments(signal),
  });
  const ciRunsQuery = useQuery({
    queryKey: ["ci-eval-runs"],
    queryFn: ({ signal }) => listCiEvalRuns(signal),
  });
  const createMutation = useMutation({
    mutationFn: () =>
      createEvalLabDataset({
        name: name.trim(),
        description: description.trim() || null,
      }),
    onSuccess: () => {
      setName("");
      setDescription("");
      setCreateOpen(false);
      void queryClient.invalidateQueries({ queryKey: ["eval-datasets"] });
    },
  });
  const datasets = datasetsQuery.data ?? [];
  const experiments = experimentsQuery.data ?? [];
  const ciRuns = ciRunsQuery.data ?? [];
  const totalCases = datasets.reduce(
    (sum, dataset) => sum + dataset.case_count,
    0,
  );
  const latestGate = experiments[0]?.gate ?? null;
  const error =
    datasetsQuery.error ?? experimentsQuery.error ?? ciRunsQuery.error;

  return (
    <section className={styles.page} aria-labelledby="quality-title">
      <WorkbenchPageHeader
        actions={
          ciRunsView ? undefined : (
            <>
              <button
                type="button"
                onClick={() => setCreateOpen((current) => !current)}
              >
                <Plus aria-hidden="true" size={16} /> New dataset
              </button>
              <Link to="/app/evals?view=ci-runs">View CI runs</Link>
            </>
          )
        }
        description={
          ciRunsView
            ? "Review automated retrieval gates by branch and commit before a regression reaches production."
            : "Define expected evidence, compare retrieval modes, and prevent quality regressions."
        }
        section="Quality"
        title={ciRunsView ? "CI Runs" : "Eval Lab"}
        titleId="quality-title"
      />

      {error ? (
        <div className={styles.alert} role="alert">
          <AlertTriangle aria-hidden="true" size={18} />
          <span>
            {error instanceof Error
              ? error.message
              : "Quality data could not be loaded."}
          </span>
        </div>
      ) : null}

      {ciRunsView ? (
        <>
          <WorkbenchWorkflowGuide
            currentStep="ci"
            impact="CI gates compare current retrieval quality against release thresholds before regressions reach production."
            nextAction={{
              label: "Open API key setup",
              to: "/app/settings?tab=api-keys",
            }}
            purpose="CI Runs show automated Eval Lab decisions by branch, commit, and config label."
          />
          <CiRunsView isLoading={ciRunsQuery.isLoading} runs={ciRuns} />
        </>
      ) : (
        <>
          <WorkbenchWorkflowGuide
            currentStep="quality"
            impact="Eval cases turn good and bad evidence into repeatable checks, so chunking or retrieval changes can be measured instead of guessed."
            nextAction={
              datasets.length > 0
                ? {
                    label: "Open first dataset",
                    to: `/app/evals/datasets/${datasets[0].id}`,
                  }
                : {
                    label: "Create dataset",
                    onClick: () => setCreateOpen(true),
                  }
            }
            purpose="Eval Lab is the measurement step: define expected evidence, run experiments, and decide whether a change is safe."
          />

          <section className={styles.stats} aria-label="Eval Lab summary">
            <WorkbenchMetricCard
              label="Datasets"
              value={String(datasets.length)}
            />
            <WorkbenchMetricCard label="Cases" value={String(totalCases)} />
            <WorkbenchMetricCard
              label="Experiments"
              value={String(experiments.length)}
            />
            <WorkbenchMetricCard
              label="Latest gate"
              tone={gateTone(latestGate?.status)}
              value={latestGate?.status ?? "Not run"}
            />
          </section>

          {createOpen ? (
            <CreateDatasetPanel
              description={description}
              isPending={createMutation.isPending}
              name={name}
              onCreate={() => createMutation.mutate()}
              onDescriptionChange={setDescription}
              onNameChange={setName}
            />
          ) : null}

          <div className={styles.grid}>
            <WorkbenchPanel
              className={styles.panel}
              description="Manage expected evidence and run retrieval experiments."
              title="Datasets"
            >
              <div className={styles.list}>
                {datasetsQuery.isLoading ? (
                  <p className={styles.empty}>Loading datasets…</p>
                ) : null}
                {datasets.map((dataset) => (
                  <Link
                    className={styles.datasetCard}
                    key={dataset.id}
                    to={`/app/evals/datasets/${dataset.id}`}
                  >
                    <div className={styles.cardHeader}>
                      <strong>{dataset.name}</strong>
                      <WorkbenchStatusPill
                        tone={gateTone(dataset.latest_gate?.status)}
                      >
                        {dataset.latest_gate?.status ?? "Not run"}
                      </WorkbenchStatusPill>
                    </div>
                    <p>{dataset.description ?? "No description"}</p>
                    <p>
                      {dataset.case_count} cases · updated{" "}
                      {formatDateTime(dataset.updated_at)}
                    </p>
                  </Link>
                ))}
                {!datasetsQuery.isLoading && datasets.length === 0 ? (
                  <WorkbenchEmptyState
                    description="Create a dataset, then add questions and the evidence a correct retrieval must find."
                    icon={FlaskConical}
                    primaryAction={{
                      label: "Create dataset",
                      onClick: () => setCreateOpen(true),
                    }}
                    secondaryAction={{
                      label: "Open Trace Debugger",
                      to: "/app/traces",
                    }}
                    title="No eval datasets"
                  />
                ) : null}
              </div>
            </WorkbenchPanel>

            <WorkbenchPanel
              className={styles.panel}
              description="Latest mode comparisons and release-gate decisions."
              title="Recent experiments"
            >
              <div className={styles.list}>
                {experiments.slice(0, 5).map((experiment) => (
                  <Link
                    className={styles.experimentCard}
                    key={experiment.id}
                    to={`/app/evals/experiments/${experiment.id}`}
                  >
                    <div className={styles.cardHeader}>
                      <strong>{experiment.name}</strong>
                      <WorkbenchStatusPill
                        icon={
                          experiment.gate.status === "passed"
                            ? CheckCircle2
                            : XCircle
                        }
                        tone={gateTone(experiment.gate.status)}
                      >
                        {experiment.gate.status}
                      </WorkbenchStatusPill>
                    </div>
                    <p>
                      {experiment.dataset_name} · {experiment.modes.join(", ")}
                    </p>
                  </Link>
                ))}
                {!experimentsQuery.isLoading && experiments.length === 0 ? (
                  <WorkbenchEmptyState
                    description="Add expected evidence to a dataset, then compare lexical, vector, and hybrid retrieval."
                    icon={FlaskConical}
                    primaryAction={
                      datasets[0]
                        ? {
                            label: "Open first dataset",
                            to: `/app/evals/datasets/${datasets[0].id}`,
                          }
                        : {
                            label: "Create dataset",
                            onClick: () => setCreateOpen(true),
                          }
                    }
                    secondaryAction={{
                      label: "Inspect saved runs",
                      to: "/app/traces",
                    }}
                    title="No experiments run"
                  />
                ) : null}
              </div>
            </WorkbenchPanel>
          </div>
        </>
      )}
    </section>
  );
}

function gateTone(
  status: string | null | undefined,
): "success" | "danger" | "neutral" {
  if (status === "passed") return "success";
  if (status === "failed") return "danger";
  return "neutral";
}
