import type { RetrievalQueryHit } from "../../../../lib/api/retrieval";
import styles from "../TraceDetailPage.module.css";

export function TraceScoreBars({ hit }: { hit: RetrievalQueryHit }) {
  const scores = [
    ["semantic", hit.normalized_score_breakdown.semantic],
    ["lexical", hit.normalized_score_breakdown.lexical],
    ["section", hit.normalized_score_breakdown.section],
  ] as const;

  return (
    <div className={styles.scoreBars} aria-label="Score breakdown">
      {scores.map(([label, value]) => (
        <div className={styles.scoreBar} key={label}>
          <span>{label}</span>
          <div className={styles.track}>
            <i style={{ width: `${Math.max(3, value * 100)}%` }} />
          </div>
        </div>
      ))}
    </div>
  );
}
