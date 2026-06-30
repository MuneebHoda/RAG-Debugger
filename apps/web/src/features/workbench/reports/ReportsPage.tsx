import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  FileBarChart,
  GitBranch,
  ScanSearch,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router-dom";

import {
  listCiEvalRuns,
  listEvalLabExperiments,
} from "../../../lib/api/evalLab";
import { listSources } from "../../../lib/api/sources";
import { listTraces } from "../../../lib/api/traces";
import { formatDateTime } from "../../../lib/dateTime";
import { ReportCreationPanel } from "./components/ReportCreationPanel";
import { CreateAuditReportAction } from "./components/CreateAuditReportAction";
import { ReportList } from "./components/ReportList";
import { useDebugReports } from "./hooks/useReports";
import styles from "./ReportsPage.module.css";

export function ReportsPage() {
  const navigate = useNavigate();
  const reportsQuery = useDebugReports();
  const sourcesQuery = useQuery({
    queryKey: ["sources"],
    queryFn: ({ signal }) => listSources(signal),
  });
  const tracesQuery = useQuery({
    queryKey: ["traces"],
    queryFn: ({ signal }) => listTraces(signal),
  });
  const experimentsQuery = useQuery({
    queryKey: ["eval-experiments"],
    queryFn: ({ signal }) => listEvalLabExperiments(signal),
  });
  const ciRunsQuery = useQuery({
    queryKey: ["ci-eval-runs"],
    queryFn: ({ signal }) => listCiEvalRuns(signal),
  });
  const sources = sourcesQuery.data ?? [];
  const traces = tracesQuery.data ?? [];
  const experiments = experimentsQuery.data ?? [];
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
  const candidateLoading =
    sourcesQuery.isLoading ||
    tracesQuery.isLoading ||
    experimentsQuery.isLoading ||
    ciRunsQuery.isLoading;
  const error =
    reportsQuery.error ??
    sourcesQuery.error ??
    tracesQuery.error ??
    experimentsQuery.error ??
    ciRunsQuery.error;

  return (
    <section className={styles.page} aria-labelledby="reports-title">
      <header className={styles.header}>
        <div>
          <p>Share</p>
          <h1 id="reports-title">Audit reports</h1>
          <span>
            Convert retrieval evidence and quality failures into defensible,
            privacy-classified diagnoses.
          </span>
        </div>
      </header>

      {error ? (
        <div className={styles.alert} role="alert">
          <AlertTriangle aria-hidden="true" size={18} />
          <span>
            {error instanceof Error
              ? error.message
              : "Report data could not be loaded."}
          </span>
        </div>
      ) : null}

      <ReportCreationPanel
        traces={traces}
        experiments={experiments}
        sourcesLoading={tracesQuery.isLoading || experimentsQuery.isLoading}
        onCreated={(reportId) => navigate(`/app/reports/${reportId}`)}
      />

      <section className={styles.panel} aria-labelledby="generated-reports">
        <div className={styles.panelHeading}>
          <div>
            <h2 id="generated-reports">Generated reports</h2>
            <p>Saved report snapshots remain tied to their original source.</p>
          </div>
          <span className={styles.count}>{reportsQuery.data?.length ?? 0}</span>
        </div>
        {reportsQuery.isLoading ? (
          <p className={styles.empty}>Loading audit reports…</p>
        ) : (
          <ReportList reports={reportsQuery.data ?? []} />
        )}
      </section>

      <div className={styles.sectionHeading}>
        <div>
          <p>Report candidates</p>
          <h2>Find the next diagnosis</h2>
        </div>
        <span>
          Review failed gates, weak runs, and corpus warnings before creating a
          report.
        </span>
      </div>

      {candidateLoading ? (
        <p className={styles.empty}>Loading report candidates…</p>
      ) : null}

      <div className={styles.grid}>
        <CandidatePanel
          icon={GitBranch}
          title="CI gate failures"
          description="Release decisions that need investigation."
          count={failedGates.length}
        >
          {failedGates.slice(0, 6).map((run) => (
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
              <CreateAuditReportAction
                compact
                source={{ sourceType: "ci_run", sourceId: run.id }}
              />
            </article>
          ))}
          {failedGates.length === 0 && !candidateLoading ? (
            <Empty text="No failed CI gates require review." />
          ) : null}
        </CandidatePanel>

        <CandidatePanel
          icon={ScanSearch}
          title="Run diagnoses"
          description="Saved retrieval runs with weak evidence or failure signals."
          count={diagnosedRuns.length}
          action="/app/traces"
        >
          {diagnosedRuns.slice(0, 6).map((trace) => (
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
          {diagnosedRuns.length === 0 && !candidateLoading ? (
            <Empty text="No saved runs currently need review." />
          ) : null}
        </CandidatePanel>
      </div>

      <CandidatePanel
        icon={FileBarChart}
        title="Corpus findings"
        description="Extraction warnings that may weaken downstream evidence."
        count={weakDocuments.length}
        action="/app/sources"
      >
        {weakDocuments.slice(0, 6).map(({ document }) => (
          <Link
            className={styles.reportRow}
            key={document.id}
            to={`/app/sources/${document.id}`}
          >
            <div className={styles.rowTop}>
              <strong>{document.path}</strong>
              <span className={styles.risk}>{document.extraction_quality}</span>
            </div>
            <p>
              {document.warnings
                .map((warning) => warning.message)
                .join(" · ") || "Low extraction quality"}
            </p>
            <small>{prettyLabel(document.profile)}</small>
          </Link>
        ))}
        {weakDocuments.length === 0 && !candidateLoading ? (
          <Empty text="No extraction findings require attention." />
        ) : null}
      </CandidatePanel>
    </section>
  );
}

function CandidatePanel({
  icon: Icon,
  title,
  description,
  count,
  action,
  children,
}: {
  icon: typeof GitBranch;
  title: string;
  description: string;
  count: number;
  action?: string;
  children: React.ReactNode;
}) {
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div className={styles.panelTitle}>
          <Icon aria-hidden="true" size={17} />
          <div>
            <h2>{title}</h2>
            <p>{description}</p>
          </div>
        </div>
        {action ? (
          <Link className={styles.textLink} to={action}>
            Open <ArrowRight aria-hidden="true" size={14} />
          </Link>
        ) : (
          <span className={styles.count}>{count}</span>
        )}
      </div>
      <div className={styles.list}>{children}</div>
    </section>
  );
}

function Empty({ text }: { text: string }) {
  return (
    <div className={styles.emptyState}>
      <CheckCircle2 aria-hidden="true" size={18} /> {text}
    </div>
  );
}

function prettyLabel(value: string) {
  return value.replaceAll("_", " ");
}
