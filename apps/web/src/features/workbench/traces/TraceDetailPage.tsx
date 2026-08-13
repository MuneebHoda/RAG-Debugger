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
import { WorkbenchStatusPill } from "../../../components/workbench/WorkbenchStatusPill";
import { WorkbenchWorkflowGuide } from "../../../components/workbench/WorkbenchWorkflowGuide";
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
            allowedPrivacyModes={allowedReportModes(trace)}
            disabledReason={
              trace.ingestion?.privacy_mode === "full_local_only"
                ? "Full-local imported traces cannot be reported or exported."
                : undefined
            }
            source={{ sourceType: "trace", sourceId: trace.id }}
          />
        }
        back={{ label: "Back to Trace Debugger", to: "/app/traces" }}
        description={trace.summary}
        metadata={
          <>
            <WorkbenchStatusPill tone="neutral">
              {trace.status}
            </WorkbenchStatusPill>
            <WorkbenchStatusPill
              tone={evidenceTone(trace.evidence_strength ?? "weak")}
            >
              {trace.evidence_strength ?? "weak"} evidence
            </WorkbenchStatusPill>
            <WorkbenchStatusPill tone="info">
              {trace.ingestion?.source.replaceAll("_", "/") ??
                trace.retrieval?.run.retrieval_mode ??
                "unknown"}
            </WorkbenchStatusPill>
            {trace.retrieval ? (
              <WorkbenchStatusPill tone="neutral">
                {trace.retrieval.run.latency_ms} ms
              </WorkbenchStatusPill>
            ) : null}
          </>
        }
        section={trace.ingestion ? "Imported RAG trace" : "Saved retrieval run"}
        title={trace.input || "Query withheld by privacy policy"}
        titleId="run-title"
      />

      <WorkbenchWorkflowGuide
        currentStep="trace"
        impact="Use the diagnosis to decide whether the fix belongs in corpus coverage, chunking, embeddings, ranking mode, citations, or eval coverage."
        nextAction={{ label: "Add to Quality", href: "#quality" }}
        purpose="Run detail explains what happened, which evidence caused it, and what to try before turning the run into an eval or audit report."
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

function allowedReportModes(trace: import("../../../lib/api/traces").Trace) {
  if (!trace.ingestion) return undefined;
  if (trace.ingestion.privacy_mode === "metadata_only") {
    return ["metadata_only"] as const;
  }
  if (trace.ingestion.privacy_mode === "snippets_allowed") {
    return ["metadata_only", "snippets_allowed"] as const;
  }
  return [] as const;
}

function evidenceTone(
  strength: "strong" | "medium" | "weak",
): "success" | "warning" | "neutral" {
  if (strength === "strong") return "success";
  if (strength === "weak") return "warning";
  return "neutral";
}
