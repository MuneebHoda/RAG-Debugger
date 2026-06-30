import { ArrowRight, CheckCircle2, CircleAlert } from "lucide-react";
import { Link } from "react-router-dom";

import type {
  DiagnosisRecommendation,
  EvidenceDiagnosisSummary,
} from "../../../../lib/api/retrieval";
import styles from "../TraceDetailPage.module.css";

export function TraceDiagnosisPanel({
  diagnosis,
}: {
  diagnosis: EvidenceDiagnosisSummary;
}) {
  return (
    <div className={styles.summaryGrid}>
      <section className={styles.diagnosis}>
        <div className={styles.diagnosisHeading}>
          <span className={styles.diagnosisLabel}>Primary diagnosis</span>
          <span className={styles[diagnosis.outcome]}>{diagnosis.outcome}</span>
        </div>
        <h2>
          {diagnosis.primary_issue?.title ?? "No failure signal detected"}
        </h2>
        <p>{diagnosis.summary}</p>
        <ul className={styles.signalList} aria-label="Failure labels">
          {diagnosis.failures.length === 0 ? (
            <li>
              <CheckCircle2 aria-hidden="true" size={16} /> No deterministic
              failure labels
            </li>
          ) : (
            diagnosis.failures.map((failure) => (
              <li key={failure.code}>
                <CircleAlert aria-hidden="true" size={16} />
                <span>
                  <strong>{failure.title}</strong>
                  <small>
                    {failure.severity}
                    {failure.evidence_refs.length > 0
                      ? ` · ${failure.evidence_refs.join(", ")}`
                      : ""}
                  </small>
                </span>
              </li>
            ))
          )}
        </ul>
      </section>

      <section className={styles.actionPanel}>
        <h2>Recommended next actions</h2>
        {diagnosis.recommendations.length === 0 ? (
          <p>Preserve this result as a Quality case to detect regressions.</p>
        ) : (
          <ol className={styles.recommendationList}>
            {diagnosis.recommendations.slice(0, 3).map((recommendation) => (
              <RecommendationItem
                key={recommendation.code}
                recommendation={recommendation}
              />
            ))}
          </ol>
        )}
      </section>
    </div>
  );
}

function RecommendationItem({
  recommendation,
}: {
  recommendation: DiagnosisRecommendation;
}) {
  return (
    <li>
      <div>
        <strong>{recommendation.title}</strong>
        <small>
          {recommendation.priority} · {recommendation.area.replaceAll("_", " ")}
        </small>
      </div>
      <p>{recommendation.action}</p>
      <Link to={recommendationRoute(recommendation)}>
        Open workflow <ArrowRight aria-hidden="true" size={14} />
      </Link>
    </li>
  );
}

function recommendationRoute(recommendation: DiagnosisRecommendation): string {
  switch (recommendation.area) {
    case "chunking":
    case "metadata_filters":
    case "corpus_coverage":
      return "/app/sources";
    case "citations":
      return "?tab=evidence";
    case "embeddings":
    case "top_k":
    case "retrieval_mode":
    case "reranking":
      return "?tab=compare";
    case "other":
      return "?tab=summary#quality";
  }
}
