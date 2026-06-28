import { Loader2, Search, SlidersHorizontal } from "lucide-react";
import type { ReactNode } from "react";

import type { RetrievalMode } from "../../../../lib/api/retrieval";
import styles from "../RetrievalPage.module.css";

const RETRIEVAL_MODES: RetrievalMode[] = ["hybrid", "vector", "lexical"];

export function RetrievalQueryPanel({
  advancedControls,
  documentCount,
  isLoadingSources,
  isQuerying,
  query,
  retrievalMode,
  topK,
  onQueryChange,
  onRetrievalModeChange,
  onSubmit,
  onTopKChange,
}: {
  advancedControls: ReactNode;
  documentCount: number;
  isLoadingSources: boolean;
  isQuerying: boolean;
  query: string;
  retrievalMode: RetrievalMode;
  topK: number;
  onQueryChange: (value: string) => void;
  onRetrievalModeChange: (mode: RetrievalMode) => void;
  onSubmit: () => void;
  onTopKChange: (value: number) => void;
}) {
  return (
    <div className={`panel ${styles.controls}`}>
      <div className="panel-heading">
        <h2>Question</h2>
        <span className="status-pill">
          {isLoadingSources ? "Loading" : `${documentCount} documents`}
        </span>
      </div>

      <label className={styles.queryField}>
        What should the corpus answer?
        <textarea
          value={query}
          onChange={(event) => onQueryChange(event.currentTarget.value)}
          placeholder="Which chunks explain the policy exception, product behavior, or technical decision?"
        />
      </label>

      <div className={styles.modeTabs} aria-label="Retrieval mode">
        {RETRIEVAL_MODES.map((mode) => (
          <button
            aria-pressed={mode === retrievalMode}
            className={
              mode === retrievalMode ? styles.activeModeTab : styles.modeTab
            }
            key={mode}
            type="button"
            onClick={() => onRetrievalModeChange(mode)}
          >
            {mode}
          </button>
        ))}
      </div>

      <button
        className={`primary-button ${styles.primaryAction}`}
        disabled={query.trim().length === 0 || isQuerying || topK <= 0}
        type="button"
        onClick={onSubmit}
      >
        {isQuerying ? (
          <Loader2 aria-hidden="true" className="spin" size={18} />
        ) : (
          <Search aria-hidden="true" size={18} />
        )}
        Run retrieval
      </button>

      <details className={styles.advanced}>
        <summary>
          <SlidersHorizontal aria-hidden="true" size={16} /> Advanced
        </summary>
        <div className={styles.advancedContent}>
          <label className={styles.topKField}>
            Results to return
            <input
              max={25}
              min={1}
              type="number"
              value={topK}
              onChange={(event) =>
                onTopKChange(Number(event.currentTarget.value))
              }
            />
          </label>
          {advancedControls}
        </div>
      </details>
    </div>
  );
}
