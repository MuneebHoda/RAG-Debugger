import { CheckCircle2, GitBranch, Plus, XCircle } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router-dom";

import {
  createEvalLabDataset,
  listCiEvalRuns,
  listEvalLabDatasets,
  listEvalLabExperiments,
} from "../../../lib/api/evalLab";
import { formatDateTime } from "../../../lib/dateTime";
import styles from "./QualityPage.module.css";

export function QualityPage() {
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
      queryClient.invalidateQueries({ queryKey: ["eval-datasets"] });
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

  return (
    <section className={styles.page} aria-labelledby="quality-title">
      <header className={styles.header}>
        <div>
          <p>Improve</p>
          <h1 id="quality-title">Quality</h1>
          <span>Expected evidence, experiments, and release gates.</span>
        </div>
        <button
          className={styles.headerButton}
          type="button"
          onClick={() => setCreateOpen((current) => !current)}
        >
          <Plus aria-hidden="true" size={16} /> New dataset
        </button>
      </header>

      <section className={styles.stats} aria-label="Quality summary">
        <Stat label="Datasets" value={String(datasets.length)} />
        <Stat label="Cases" value={String(totalCases)} />
        <Stat label="Experiments" value={String(experiments.length)} />
        <Stat label="Latest gate" value={latestGate?.status ?? "Not run"} />
      </section>

      {createOpen ? (
        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div>
              <h2>Create a quality dataset</h2>
              <p>
                Group questions that must keep retrieving the right evidence.
              </p>
            </div>
          </div>
          <div className={styles.form}>
            <div className={styles.formGrid}>
              <label>
                Dataset name
                <input
                  value={name}
                  onChange={(event) => setName(event.currentTarget.value)}
                />
              </label>
              <label>
                Description
                <input
                  value={description}
                  onChange={(event) =>
                    setDescription(event.currentTarget.value)
                  }
                />
              </label>
            </div>
            <button
              className={styles.primaryButton}
              disabled={!name.trim() || createMutation.isPending}
              type="button"
              onClick={() => createMutation.mutate()}
            >
              Create dataset
            </button>
          </div>
        </section>
      ) : null}

      <div className={styles.grid}>
        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div>
              <h2>Datasets</h2>
              <p>Open a dataset to manage cases and run an experiment.</p>
            </div>
          </div>
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
                  <span
                    className={styles[dataset.latest_gate?.status ?? "neutral"]}
                  >
                    {dataset.latest_gate?.status ?? "Not run"}
                  </span>
                </div>
                <p>{dataset.description ?? "No description"}</p>
                <p>
                  {dataset.case_count} cases · updated{" "}
                  {formatDateTime(dataset.updated_at)}
                </p>
              </Link>
            ))}
          </div>
        </section>

        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div>
              <h2>Recent experiments</h2>
              <p>Latest release-gate decisions.</p>
            </div>
          </div>
          <div className={styles.list}>
            {experiments.slice(0, 5).map((experiment) => (
              <Link
                className={styles.experimentCard}
                key={experiment.id}
                to={`/app/evals/experiments/${experiment.id}`}
              >
                <div className={styles.cardHeader}>
                  <strong>{experiment.name}</strong>
                  <span className={styles[experiment.gate.status]}>
                    {experiment.gate.status === "passed" ? (
                      <CheckCircle2 aria-hidden="true" size={13} />
                    ) : (
                      <XCircle aria-hidden="true" size={13} />
                    )}
                    {experiment.gate.status}
                  </span>
                </div>
                <p>
                  {experiment.dataset_name} · {experiment.modes.join(", ")}
                </p>
              </Link>
            ))}
            {experiments.length === 0 ? (
              <p className={styles.empty}>No experiments have run yet.</p>
            ) : null}
          </div>
        </section>
      </div>

      <section className={styles.panel}>
        <div className={styles.panelHeading}>
          <div>
            <h2>CI gates</h2>
            <p>Automated quality checks from branches and commits.</p>
          </div>
          <GitBranch aria-hidden="true" size={18} />
        </div>
        <div className={styles.list}>
          {ciRuns.slice(0, 5).map((run) => (
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
          {ciRuns.length === 0 ? (
            <p className={styles.empty}>No CI gates have run yet.</p>
          ) : null}
        </div>
      </section>
    </section>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.stat}>
      <small>{label}</small>
      <strong>{value}</strong>
    </div>
  );
}
