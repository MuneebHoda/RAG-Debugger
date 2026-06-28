import {
  AlertTriangle,
  ArrowLeft,
  FileSearch,
  GitCompare,
  ListTree,
  ScanSearch,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams, useSearchParams } from "react-router-dom";

import { getTrace } from "../../../lib/api/traces";
import {
  TraceComparePanel,
  TraceEvidencePanel,
  TraceSummaryPanel,
  TraceTimelinePanel,
} from "./TraceDetailPanels";
import styles from "./TraceDetailPage.module.css";

const tabs = [
  { id: "summary", label: "Summary", icon: ScanSearch },
  { id: "evidence", label: "Evidence", icon: FileSearch },
  { id: "timeline", label: "Timeline", icon: ListTree },
  { id: "compare", label: "Compare", icon: GitCompare },
] as const;

type TraceTab = (typeof tabs)[number]["id"];

export function TraceDetailPage() {
  const { traceId } = useParams<{ traceId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const tabParam = searchParams.get("tab");
  const activeTab: TraceTab = tabs.some((tab) => tab.id === tabParam)
    ? (tabParam as TraceTab)
    : "summary";
  const traceQuery = useQuery({
    queryKey: ["trace", traceId],
    queryFn: ({ signal }) => getTrace(traceId!, signal),
    enabled: Boolean(traceId),
  });

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
        <Link className={styles.backLink} to="/app/traces">
          <ArrowLeft aria-hidden="true" size={15} /> Back to Runs
        </Link>
      </section>
    );
  }

  const trace = traceQuery.data;
  return (
    <section className={styles.page} aria-labelledby="run-title">
      <Link className={styles.backLink} to="/app/traces">
        <ArrowLeft aria-hidden="true" size={15} /> Back to Runs
      </Link>

      <header className={styles.header}>
        <div>
          <h1 id="run-title">{trace.input}</h1>
          <p>{trace.summary}</p>
        </div>
        <div className={styles.headerMeta}>
          <span className={styles.status}>{trace.status}</span>
          <span className={styles[trace.evidence_strength ?? "weak"]}>
            {trace.evidence_strength ?? "weak"} evidence
          </span>
          <span className={styles.metaPill}>
            {trace.retrieval?.run.retrieval_mode ?? "unknown"}
          </span>
          <span className={styles.metaPill}>
            {trace.retrieval?.run.latency_ms ?? 0} ms
          </span>
        </div>
      </header>

      <div className={styles.tabs} role="tablist" aria-label="Run details">
        {tabs.map((tab) => (
          <button
            aria-selected={activeTab === tab.id}
            className={activeTab === tab.id ? styles.activeTab : styles.tab}
            key={tab.id}
            role="tab"
            type="button"
            onClick={() => setSearchParams({ tab: tab.id })}
          >
            <tab.icon aria-hidden="true" size={16} /> {tab.label}
          </button>
        ))}
      </div>

      {activeTab === "summary" ? <TraceSummaryPanel trace={trace} /> : null}
      {activeTab === "evidence" ? <TraceEvidencePanel trace={trace} /> : null}
      {activeTab === "timeline" ? <TraceTimelinePanel trace={trace} /> : null}
      {activeTab === "compare" ? <TraceComparePanel trace={trace} /> : null}
    </section>
  );
}
