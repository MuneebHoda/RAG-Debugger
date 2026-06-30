import {
  AlertTriangle,
  ArrowLeft,
  Check,
  Clipboard,
  FileLock2,
} from "lucide-react";
import { Link, useParams } from "react-router-dom";

import { formatDateTime } from "../../../lib/dateTime";
import { useCopyReportMarkdown, useDebugReport } from "./hooks/useReports";
import styles from "./ReportDetailPage.module.css";

export function ReportDetailPage() {
  const { reportId } = useParams<{ reportId: string }>();
  const reportQuery = useDebugReport(reportId);
  const copyMarkdown = useCopyReportMarkdown();

  if (reportQuery.isLoading) {
    return <p className={styles.state}>Loading audit report…</p>;
  }

  if (reportQuery.isError || !reportQuery.data) {
    return (
      <section className={styles.errorState} role="alert">
        <AlertTriangle aria-hidden="true" size={22} />
        <strong>This audit report could not be opened.</strong>
        <button type="button" onClick={() => void reportQuery.refetch()}>
          Retry
        </button>
        <Link to="/app/reports">Back to reports</Link>
      </section>
    );
  }

  const report = reportQuery.data;
  const exportBlocked = report.privacy_mode === "full_local_only";

  return (
    <section className={styles.page} aria-labelledby="report-title">
      <Link className={styles.backLink} to="/app/reports">
        <ArrowLeft aria-hidden="true" size={15} /> Back to reports
      </Link>

      <header className={styles.header}>
        <div>
          <p>{prettyLabel(report.source.type)} audit</p>
          <h1 id="report-title">{report.title}</h1>
          <span>{formatDateTime(report.created_at)}</span>
        </div>
        <button
          type="button"
          className={styles.copyButton}
          disabled={exportBlocked || copyMarkdown.isPending}
          onClick={() => copyMarkdown.mutate(report.id)}
        >
          {copyMarkdown.isSuccess ? (
            <Check aria-hidden="true" size={16} />
          ) : (
            <Clipboard aria-hidden="true" size={16} />
          )}
          {copyMarkdown.isSuccess
            ? "Copied"
            : copyMarkdown.isPending
              ? "Copying…"
              : "Copy Markdown"}
        </button>
      </header>

      <section className={styles.privacyBanner}>
        <FileLock2 aria-hidden="true" size={18} />
        <div>
          <strong>{prettyLabel(report.privacy_mode)}</strong>
          <p>{privacyMessage(report.privacy_mode)}</p>
        </div>
      </section>

      {copyMarkdown.isError ? (
        <p className={styles.inlineError} role="alert">
          {copyMarkdown.error instanceof Error
            ? copyMarkdown.error.message
            : "Markdown could not be copied."}
        </p>
      ) : null}

      <section className={styles.summary}>
        <p>Executive summary</p>
        <h2>{report.executive_summary}</h2>
        {report.subject ? <span>{report.subject}</span> : null}
      </section>

      <div className={styles.twoColumn}>
        <ReportSection title="System and configuration">
          <dl className={styles.contextList}>
            {Object.entries(report.context).map(([key, value]) => (
              <div key={key}>
                <dt>{prettyLabel(key)}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
        </ReportSection>

        <ReportSection title="Findings">
          <div className={styles.stack}>
            {report.findings.map((finding) => (
              <article className={styles.finding} key={finding.code}>
                <header>
                  <span className={styles[finding.severity]}>
                    {finding.severity}
                  </span>
                  <code>{finding.code}</code>
                </header>
                <strong>{finding.title}</strong>
                <p>{finding.summary}</p>
                {finding.failure_labels.length > 0 ? (
                  <footer>
                    {finding.failure_labels.map((label) => (
                      <span key={label}>{prettyLabel(label)}</span>
                    ))}
                  </footer>
                ) : null}
              </article>
            ))}
            {report.findings.length === 0 ? (
              <p className={styles.empty}>No failure findings were recorded.</p>
            ) : null}
          </div>
        </ReportSection>
      </div>

      <ReportSection title="Evidence references">
        <div className={styles.evidenceGrid}>
          {report.evidence.map((evidence) => (
            <article className={styles.evidence} key={evidence.label}>
              <header>
                <strong>{evidence.label}</strong>
                <span>{evidence.role}</span>
              </header>
              <dl>
                <div>
                  <dt>Rank</dt>
                  <dd>{evidence.rank ?? "—"}</dd>
                </div>
                <div>
                  <dt>Chunk</dt>
                  <dd>{evidence.chunk_id ?? "Not recorded"}</dd>
                </div>
                <div>
                  <dt>Strength</dt>
                  <dd>{evidence.evidence_strength ?? "Not scored"}</dd>
                </div>
                <div>
                  <dt>Checksum</dt>
                  <dd>{evidence.checksum_prefix ?? "—"}</dd>
                </div>
              </dl>
              {evidence.snippet ? (
                <blockquote>{evidence.snippet}</blockquote>
              ) : null}
              {[
                ...evidence.chunk_quality_flags,
                ...evidence.retrieval_quality_flags,
              ].length > 0 ? (
                <footer>
                  {[
                    ...evidence.chunk_quality_flags,
                    ...evidence.retrieval_quality_flags,
                  ].map((flag) => (
                    <span key={flag}>{prettyLabel(flag)}</span>
                  ))}
                </footer>
              ) : null}
            </article>
          ))}
          {report.evidence.length === 0 ? (
            <p className={styles.empty}>
              No evidence references were available.
            </p>
          ) : null}
        </div>
      </ReportSection>

      <ReportSection title="Prioritized recommendations">
        <ol className={styles.recommendations}>
          {report.recommendations.map((recommendation) => (
            <li key={recommendation.code}>
              <span className={styles.priority}>{recommendation.priority}</span>
              <div>
                <small>{prettyLabel(recommendation.area)}</small>
                <strong>{recommendation.title}</strong>
                <p>{recommendation.rationale}</p>
                <b>Action: {recommendation.action}</b>
              </div>
            </li>
          ))}
        </ol>
      </ReportSection>
    </section>
  );
}

function ReportSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className={styles.section}>
      <h2>{title}</h2>
      {children}
    </section>
  );
}

function privacyMessage(mode: string) {
  if (mode === "full_local_only") {
    return "This report may contain unrestricted local diagnostics. Export is blocked until it is redacted.";
  }
  if (mode === "snippets_allowed") {
    return "This report may include explicitly approved, bounded evidence snippets.";
  }
  return "This report contains identifiers, metrics, labels, and recommendations without raw query or document content.";
}

function prettyLabel(value: string) {
  return value.replaceAll("_", " ");
}
