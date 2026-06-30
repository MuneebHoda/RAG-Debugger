import { ArrowRight } from "lucide-react";
import { Link } from "react-router-dom";

import type { Trace } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";
import { recommendationFor } from "../utils/traceLabels";
import { SaveToQualityPanel } from "./SaveToQualityPanel";
import { TraceFailureLabels } from "./TraceFailureLabels";

export function TraceSummaryPanel({ trace }: { trace: Trace }) {
  const recommendation = recommendationFor(trace.failure_labels);
  return (
    <div className={styles.stack}>
      <div className={styles.summaryGrid}>
        <section className={styles.diagnosis}>
          <span className={styles.diagnosisLabel}>What happened</span>
          <h2>{trace.summary}</h2>
          <p>
            {trace.failure_labels.length === 0
              ? "CorpusLab found usable evidence without a deterministic failure signal."
              : "CorpusLab found the following likely causes."}
          </p>
          <TraceFailureLabels labels={trace.failure_labels} />
        </section>

        <section className={styles.actionPanel}>
          <h2>Recommended next action</h2>
          <p>{recommendation.detail}</p>
          <Link to={recommendation.route}>
            {recommendation.label} <ArrowRight aria-hidden="true" size={16} />
          </Link>
        </section>
      </div>

      <section className={styles.panel}>
        <div className={styles.panelHeading}>
          <div>
            <h2>Evidence summary</h2>
            <p>The answer produced from the strongest retrieved excerpts.</p>
          </div>
        </div>
        <p className={styles.answer}>
          {trace.output ??
            trace.retrieval?.answer.text ??
            "No answer was produced."}
        </p>
      </section>

      <SaveToQualityPanel trace={trace} />
    </div>
  );
}
