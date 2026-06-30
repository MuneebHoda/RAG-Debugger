import type { Trace } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";
import { SaveToQualityPanel } from "./SaveToQualityPanel";
import { TraceDiagnosisPanel } from "./TraceDiagnosisPanel";
import { TraceFailureLabels } from "./TraceFailureLabels";

export function TraceSummaryPanel({ trace }: { trace: Trace }) {
  return (
    <div className={styles.stack}>
      {trace.diagnosis ? (
        <TraceDiagnosisPanel diagnosis={trace.diagnosis} />
      ) : (
        <section className={styles.diagnosis}>
          <span className={styles.diagnosisLabel}>Legacy diagnosis</span>
          <h2>{trace.summary}</h2>
          <p>This saved run uses the earlier failure-label format.</p>
          <TraceFailureLabels labels={trace.failure_labels} />
        </section>
      )}

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
