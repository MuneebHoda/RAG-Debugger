import type {
  EvidenceScoreExplanation,
  RetrievalQueryHit,
} from "../../../../lib/api/retrieval";
import styles from "../TraceDetailPage.module.css";

export function TraceScoreBars({
  hit,
  explanation,
}: {
  hit: RetrievalQueryHit;
  explanation?: EvidenceScoreExplanation;
}) {
  const scores = [
    ["semantic", hit.normalized_score_breakdown.semantic],
    ["lexical", hit.normalized_score_breakdown.lexical],
    ["phrase", hit.normalized_score_breakdown.phrase],
    ["section", hit.normalized_score_breakdown.section],
    ["path", hit.normalized_score_breakdown.path],
    ["metadata", hit.normalized_score_breakdown.metadata],
  ] as const;

  return (
    <div className={styles.scoreExplanation}>
      {explanation ? (
        <div className={styles.scoreSummary}>
          <strong>{explanation.summary}</strong>
          <span>
            dominant {explanation.dominant_signal.replaceAll("_", " ")}
            {explanation.score_delta_to_next === null
              ? ""
              : ` · ${explanation.score_delta_to_next.toFixed(2)} ahead of next`}
          </span>
        </div>
      ) : null}
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
    </div>
  );
}
