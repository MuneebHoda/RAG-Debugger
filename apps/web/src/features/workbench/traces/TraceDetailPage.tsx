import {
  AlertTriangle,
  ArrowLeft,
  FileSearch,
  GitCompare,
  ListTree,
  ScanSearch,
} from "lucide-react";
import { Link } from "react-router-dom";

import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import { CreateAuditReportAction } from "../reports/components/CreateAuditReportAction";
import { TraceEvidencePanel } from "./components/TraceEvidencePanel";
import { TraceRerunPanel } from "./components/TraceRerunPanel";
import { TraceSummaryPanel } from "./components/TraceSummaryPanel";
import { TraceTimeline } from "./components/TraceTimeline";
import { useTraceDebugger } from "./hooks/useTraceDebugger";
import styles from "./TraceDetailPage.module.css";

const tabs = [
  { id: "summary", label: "Summary", icon: ScanSearch },
  { id: "evidence", label: "Evidence", icon: FileSearch },
  { id: "timeline", label: "Timeline", icon: ListTree },
  { id: "compare", label: "Compare", icon: GitCompare },
] as const;

export function TraceDetailPage() {
  const { activeTab, selectTab, traceQuery } = useTraceDebugger();

  if (traceQuery.isLoading) {
    return <div className={styles.loading}>Loading run diagnosis…</div>;
  }

  if (traceQuery.isError || !traceQuery.data) {
    return (
      <section className={styles.errorState} role="alert">
        <AlertTriangle aria-hidden="true" size={24} />
        <strong>This run could not be opened</strong>
        <span>
          The run may have been removed or its data may be unavailable.
        </span>
        <button type="button" onClick={() => void traceQuery.refetch()}>
          Retry
        </button>
        <Link className="secondary-button compact" to="/app/traces">
          <ArrowLeft aria-hidden="true" size={15} /> Back to Runs
        </Link>
      </section>
    );
  }

  const trace = traceQuery.data;
  return (
    <section className={styles.page} aria-labelledby="run-title">
      <WorkbenchPageHeader
        actions={
          <CreateAuditReportAction
            compact
            source={{ sourceType: "trace", sourceId: trace.id }}
          />
        }
        back={{ label: "Back to Trace Debugger", to: "/app/traces" }}
        description={trace.summary}
        metadata={
          <>
            <span className="status-pill">{trace.status}</span>
            <span
              className={`status-pill ${styles[trace.evidence_strength ?? "weak"]}`}
            >
              {trace.evidence_strength ?? "weak"} evidence
            </span>
            <span className="status-pill">
              {trace.retrieval?.run.retrieval_mode ?? "unknown"}
            </span>
            <span className="status-pill">
              {trace.retrieval?.run.latency_ms ?? 0} ms
            </span>
          </>
        }
        section="Saved retrieval run"
        title={trace.input}
        titleId="run-title"
      />

      <div className={styles.tabs} role="tablist" aria-label="Run details">
        {tabs.map((tab) => (
          <button
            aria-selected={activeTab === tab.id}
            className={activeTab === tab.id ? styles.activeTab : styles.tab}
            key={tab.id}
            role="tab"
            type="button"
            onClick={() => selectTab(tab.id)}
          >
            <tab.icon aria-hidden="true" size={16} /> {tab.label}
          </button>
        ))}
      </div>

      {activeTab === "summary" ? <TraceSummaryPanel trace={trace} /> : null}
      {activeTab === "evidence" ? <TraceEvidencePanel trace={trace} /> : null}
      {activeTab === "timeline" ? <TraceTimeline trace={trace} /> : null}
      {activeTab === "compare" ? <TraceRerunPanel trace={trace} /> : null}
    </section>
  );
}
