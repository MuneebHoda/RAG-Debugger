import { ArrowRight, Search } from "lucide-react";
import { Link } from "react-router-dom";

import type { TraceSummary } from "../../../../lib/api/traces";
import { formatDateTime } from "../../../../lib/dateTime";
import styles from "../RunsPage.module.css";

export function TraceList({ runs }: { runs: TraceSummary[] }) {
  if (runs.length === 0) {
    return (
      <div className={styles.empty}>
        <Search aria-hidden="true" size={22} />
        <strong>No runs match this view</strong>
      </div>
    );
  }

  return (
    <div className={styles.table} role="table" aria-label="Saved runs">
      <div className={styles.tableHeader} role="row">
        <span>Question</span>
        <span>Mode</span>
        <span>Evidence</span>
        <span>Latency</span>
        <span>Created</span>
        <span />
      </div>
      {runs.map((run) => (
        <Link className={styles.row} key={run.id} to={`/app/traces/${run.id}`}>
          <span className={styles.query}>
            <strong>{run.query}</strong>
            <small>
              {run.failure_labels.length === 0
                ? "No failure signals"
                : `${run.failure_labels.length} signals · ${run.rerun_count} comparisons`}
            </small>
          </span>
          <span className={styles.pill}>{run.retrieval_mode}</span>
          <span className={styles[run.evidence_strength]}>
            {run.evidence_strength}
          </span>
          <span className={styles.cell}>{run.latency_ms} ms</span>
          <span className={styles.cell}>{formatDateTime(run.created_at)}</span>
          <ArrowRight aria-hidden="true" size={16} />
        </Link>
      ))}
    </div>
  );
}
