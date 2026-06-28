import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  Download,
  FileBarChart,
  GitBranch,
  ScanSearch,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

import { listCiEvalRuns } from "../lib/api/evalLab";
import { listSources } from "../lib/api/sources";
import { listTraces } from "../lib/api/traces";
import { formatDateTime } from "../lib/dateTime";
import styles from "./ReportsPage.module.css";

export function ReportsPage() {
  const sourcesQuery = useQuery({
    queryKey: ["sources"],
    queryFn: ({ signal }) => listSources(signal),
  });
  const tracesQuery = useQuery({
    queryKey: ["traces"],
    queryFn: ({ signal }) => listTraces(signal),
  });
  const ciRunsQuery = useQuery({
    queryKey: ["ci-eval-runs"],
    queryFn: ({ signal }) => listCiEvalRuns(signal),
  });
  const sources = sourcesQuery.data ?? [];
  const traces = tracesQuery.data ?? [];
  const ciRuns = ciRunsQuery.data ?? [];
  const failedGates = ciRuns.filter((run) => run.gate_status === "failed");
  const diagnosedRuns = traces.filter(
    (trace) =>
      trace.failure_labels.length > 0 || trace.evidence_strength === "weak",
  );
  const weakDocuments = sources
    .flatMap((source) => source.documents)
    .filter(
      ({ document }) =>
        document.extraction_quality === "low" || document.warnings.length > 0,
    );
  const isLoading =
    sourcesQuery.isLoading || tracesQuery.isLoading || ciRunsQuery.isLoading;
  const error = sourcesQuery.error ?? tracesQuery.error ?? ciRunsQuery.error;

  return (
    <section className={styles.page} aria-labelledby="reports-title">
      <header className={styles.header}>
        <div>
          <p>Share</p>
          <h1 id="reports-title">Reports</h1>
          <span>
            Review diagnoses and release-gate failures before sharing them.
          </span>
        </div>
        <button
          className={styles.secondaryButton}
          type="button"
          onClick={() => window.print()}
        >
          <Download aria-hidden="true" size={16} /> Export view
        </button>
      </header>

      {error ? (
        <div className={styles.alert} role="alert">
          <AlertTriangle aria-hidden="true" size={18} />
          <span>
            {error instanceof Error
              ? error.message
              : "Reports could not be loaded."}
          </span>
        </div>
      ) : null}

      <section className={styles.summary} aria-label="Report summary">
        <SummaryCard
          icon={GitBranch}
          label="Failed CI gates"
          value={failedGates.length}
          tone={failedGates.length ? "risk" : "ready"}
        />
        <SummaryCard
          icon={ScanSearch}
          label="Runs needing review"
          value={diagnosedRuns.length}
          tone={diagnosedRuns.length ? "risk" : "ready"}
        />
        <SummaryCard
          icon={FileBarChart}
          label="Corpus findings"
          value={weakDocuments.length}
          tone={weakDocuments.length ? "risk" : "ready"}
        />
      </section>

      {isLoading ? <p className={styles.empty}>Loading report data…</p> : null}

      <div className={styles.grid}>
        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div>
              <h2>CI gate failures</h2>
              <p>Release decisions that need investigation.</p>
            </div>
            <span className={styles.count}>{failedGates.length}</span>
          </div>
          <div className={styles.list}>
            {failedGates.slice(0, 8).map((run) => (
              <article className={styles.reportRow} key={run.id}>
                <div className={styles.rowTop}>
                  <strong>{run.report.title}</strong>
                  <span className={styles.failed}>Failed</span>
                </div>
                <p>{run.report.summary}</p>
                <small>
                  {run.branch ?? "Manual run"} ·{" "}
                  {run.commit_sha?.slice(0, 8) ?? "no commit"} ·{" "}
                  {formatDateTime(run.created_at)}
                </small>
              </article>
            ))}
            {failedGates.length === 0 && !isLoading ? (
              <Empty
                icon={CheckCircle2}
                text="No failed CI gates require review."
              />
            ) : null}
          </div>
        </section>

        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div>
              <h2>Run diagnoses</h2>
              <p>Saved retrieval runs with weak evidence or failure signals.</p>
            </div>
            <Link className={styles.textLink} to="/app/traces">
              All runs <ArrowRight aria-hidden="true" size={14} />
            </Link>
          </div>
          <div className={styles.list}>
            {diagnosedRuns.slice(0, 8).map((trace) => (
              <Link
                className={styles.reportRow}
                key={trace.id}
                to={`/app/traces/${trace.id}`}
              >
                <div className={styles.rowTop}>
                  <strong>{trace.query}</strong>
                  <span className={styles.risk}>{trace.evidence_strength}</span>
                </div>
                <p>
                  {trace.failure_labels.length > 0
                    ? trace.failure_labels.map(prettyLabel).join(" · ")
                    : "Weak evidence"}
                </p>
                <small>
                  {trace.retrieval_mode} · {trace.latency_ms} ms ·{" "}
                  {formatDateTime(trace.created_at)}
                </small>
              </Link>
            ))}
            {diagnosedRuns.length === 0 && !isLoading ? (
              <Empty
                icon={CheckCircle2}
                text="No saved runs currently need review."
              />
            ) : null}
          </div>
        </section>
      </div>

      <section className={styles.panel}>
        <div className={styles.panelHeading}>
          <div>
            <h2>Corpus findings</h2>
            <p>Extraction warnings that may weaken downstream evidence.</p>
          </div>
          <Link className={styles.textLink} to="/app/sources">
            Open Corpus <ArrowRight aria-hidden="true" size={14} />
          </Link>
        </div>
        <div className={styles.list}>
          {weakDocuments.slice(0, 8).map(({ document }) => (
            <Link
              className={styles.reportRow}
              key={document.id}
              to={`/app/sources/${document.id}`}
            >
              <div className={styles.rowTop}>
                <strong>{document.path}</strong>
                <span className={styles.risk}>
                  {document.extraction_quality}
                </span>
              </div>
              <p>
                {document.warnings
                  .map((warning) => warning.message)
                  .join(" · ") || "Low extraction quality"}
              </p>
              <small>{prettyLabel(document.profile)}</small>
            </Link>
          ))}
          {weakDocuments.length === 0 && !isLoading ? (
            <Empty
              icon={CheckCircle2}
              text="No extraction findings require attention."
            />
          ) : null}
        </div>
      </section>
    </section>
  );
}

function SummaryCard({
  icon: Icon,
  label,
  value,
  tone,
}: {
  icon: typeof GitBranch;
  label: string;
  value: number;
  tone: "risk" | "ready";
}) {
  return (
    <article className={styles.summaryCard}>
      <Icon aria-hidden="true" size={18} />
      <span>
        <strong>{value}</strong>
        <small>{label}</small>
      </span>
      <i className={styles[tone]}>{tone === "ready" ? "Ready" : "Review"}</i>
    </article>
  );
}

function Empty({
  icon: Icon,
  text,
}: {
  icon: typeof CheckCircle2;
  text: string;
}) {
  return (
    <div className={styles.emptyState}>
      <Icon aria-hidden="true" size={18} /> {text}
    </div>
  );
}

function prettyLabel(value: string) {
  return value.replaceAll("_", " ");
}
