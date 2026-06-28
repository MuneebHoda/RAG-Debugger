import { CheckCircle2, CircleAlert } from "lucide-react";

import type { FailureLabel } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";
import { FAILURE_LABELS } from "../utils/traceLabels";

export function TraceFailureLabels({ labels }: { labels: FailureLabel[] }) {
  return (
    <ul className={styles.signalList}>
      {labels.length === 0 ? (
        <li>
          <CheckCircle2 aria-hidden="true" size={16} /> No failure signals
        </li>
      ) : (
        labels.map((label) => (
          <li key={label}>
            <CircleAlert aria-hidden="true" size={16} />
            {FAILURE_LABELS[label] ?? label}
          </li>
        ))
      )}
    </ul>
  );
}
