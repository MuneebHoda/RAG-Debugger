import { ArrowRight, FileCheck2 } from "lucide-react";
import { Link } from "react-router-dom";

import type { DebugReport } from "../../../../lib/api/reports";
import { formatDateTime } from "../../../../lib/dateTime";
import styles from "../ReportsPage.module.css";

export function ReportList({ reports }: { reports: DebugReport[] }) {
  if (reports.length === 0) {
    return (
      <div className={styles.emptyState}>
        <FileCheck2 aria-hidden="true" size={18} />
        <span>
          <strong>No audit reports yet.</strong>
          Create one from a saved run or experiment to establish a shareable
          diagnosis.
        </span>
        <Link to="/app">Continue the guided demo</Link>
      </div>
    );
  }

  return (
    <div className={styles.reportList}>
      {reports.map((report) => (
        <Link
          className={styles.generatedReport}
          key={report.id}
          to={`/app/reports/${report.id}`}
        >
          <div>
            <span className={styles.sourceLabel}>{sourceLabel(report)}</span>
            <strong>{report.title}</strong>
            <p>{report.executive_summary}</p>
          </div>
          <footer>
            <span className={styles[privacyTone(report.privacy_mode)]}>
              {prettyLabel(report.privacy_mode)}
            </span>
            <small>{formatDateTime(report.created_at)}</small>
            <ArrowRight aria-hidden="true" size={15} />
          </footer>
        </Link>
      ))}
    </div>
  );
}

function sourceLabel(report: DebugReport) {
  return prettyLabel(report.source.type);
}

function privacyTone(mode: DebugReport["privacy_mode"]) {
  return mode === "full_local_only" ? "localBadge" : "privacyBadge";
}

function prettyLabel(value: string) {
  return value.replaceAll("_", " ");
}
