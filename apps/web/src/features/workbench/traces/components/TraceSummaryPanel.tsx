import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import type { Trace } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";
import { SaveToQualityPanel } from "./SaveToQualityPanel";
import { TraceDiagnosisPanel } from "./TraceDiagnosisPanel";
import { TraceFailureLabels } from "./TraceFailureLabels";

export function TraceSummaryPanel({ trace }: { trace: Trace }) {
  return (
    <div className={styles.stack}>
      {trace.diagnosis ? (
        <TraceDiagnosisPanel
          answerStatus={trace.retrieval?.answer.status}
          diagnosis={trace.diagnosis}
        />
      ) : (
        <section className={styles.diagnosis}>
          <span className={styles.diagnosisLabel}>Legacy diagnosis</span>
          <h2>{trace.summary}</h2>
          <p>This saved run uses the earlier failure-label format.</p>
          <TraceFailureLabels labels={trace.failure_labels} />
        </section>
      )}

      <WorkbenchPanel
        className={styles.panel}
        description="The answer produced from the strongest retrieved excerpts."
        title="Evidence summary"
      >
        <p className={styles.answer}>
          {trace.output ??
            trace.retrieval?.answer.text ??
            "No answer was produced."}
        </p>
      </WorkbenchPanel>

      <SaveToQualityPanel trace={trace} />
    </div>
  );
}
