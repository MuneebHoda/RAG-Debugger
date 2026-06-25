import { ArrowRight, Database } from "lucide-react";
import { Link } from "react-router-dom";

import type { OverviewAction } from "../../lib/api/overview";
import styles from "./EmptyState.module.css";

export function EmptyState({ action }: { action: OverviewAction | null }) {
  return (
    <section className={styles.empty} aria-label="Empty corpus state">
      <Database aria-hidden="true" size={28} />
      <div>
        <p>Start the control loop</p>
        <h2>No corpus data yet</h2>
        <span>
          Ingest documents to unlock extraction diagnostics, chunk inspection,
          embeddings, retrieval traces, evals, and reports.
        </span>
      </div>
      {action ? (
        <Link to={action.route}>
          {action.label}
          <ArrowRight aria-hidden="true" size={16} />
        </Link>
      ) : null}
    </section>
  );
}
