import type { LucideIcon } from "lucide-react";

import type { OverviewMetric } from "../../lib/apiClient";
import styles from "./MetricTile.module.css";

export function MetricTile({
  metric,
  icon: Icon,
}: {
  metric: OverviewMetric;
  icon: LucideIcon;
}) {
  return (
    <article className={`${styles.tile} ${styles[metric.tone]}`}>
      <div className={styles.heading}>
        <Icon aria-hidden="true" size={18} />
        <span>{metric.label}</span>
      </div>
      <strong>{metric.value}</strong>
      <small>{metric.detail}</small>
    </article>
  );
}
