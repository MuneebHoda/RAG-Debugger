import { Database, Loader2, RefreshCw } from "lucide-react";

import type { EmbeddingStatus } from "../../../../lib/api/embeddings";
import styles from "../RetrievalPage.module.css";

export function EmbeddingPanel({
  status,
  isIndexing,
  onIndex,
}: {
  status: EmbeddingStatus | null;
  isIndexing: boolean;
  onIndex: () => void;
}) {
  const readiness = embeddingReadiness(status);

  return (
    <div className={styles.embeddingStatus}>
      <div>
        <div className={styles.filterHeading}>
          <Database aria-hidden="true" size={16} />
          <strong>Embeddings</strong>
        </div>
        <small>
          {status
            ? `${status.indexed_chunks}/${status.total_chunks} indexed · ${status.model.model_name}`
            : "Status unavailable"}
        </small>
      </div>
      <span className="status-pill">{readiness}</span>
      <button
        className="secondary-button"
        disabled={isIndexing || status?.total_chunks === 0}
        type="button"
        onClick={onIndex}
      >
        {isIndexing ? (
          <Loader2 aria-hidden="true" className="spin" size={16} />
        ) : (
          <RefreshCw aria-hidden="true" size={16} />
        )}
        Index
      </button>
    </div>
  );
}

function embeddingReadiness(status: EmbeddingStatus | null) {
  if (!status) return "Unknown";
  if (status.total_chunks === 0) return "No chunks";
  if (status.missing_chunks === 0 && status.stale_chunks === 0) return "Ready";
  return "Needs index";
}
