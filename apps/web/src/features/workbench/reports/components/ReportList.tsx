import { ArrowRight, FileCheck2 } from "lucide-react";
import { Link } from "react-router-dom";

import { WorkbenchEmptyState } from "../../../../components/workbench/WorkbenchEmptyState";
import { WorkbenchStatusPill } from "../../../../components/workbench/WorkbenchStatusPill";
import type { DebugReport } from "../../../../lib/api/reports";
import { formatDateTime } from "../../../../lib/dateTime";
import styles from "../ReportsPage.module.css";

export function ReportList({ reports }: { reports: DebugReport[] }) {
  if (reports.length === 0) {
    return (
      <WorkbenchEmptyState
        description="Create a privacy-classified report from a saved retrieval run or Eval Lab experiment."
        icon={FileCheck2}
        primaryAction={{ label: "Open Trace Debugger", to: "/app/traces" }}
        secondaryAction={{ label: "Open Eval Lab", to: "/app/evals" }}
        title="No audit reports"
      />
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
            <WorkbenchStatusPill tone={privacyTone(report.privacy_mode)}>
              {prettyLabel(report.privacy_mode)}
            </WorkbenchStatusPill>
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

function privacyTone(mode: DebugReport["privacy_mode"]): "warning" | "success" {
  return mode === "full_local_only" ? "warning" : "success";
}

function prettyLabel(value: string) {
  return value.replaceAll("_", " ");
}
