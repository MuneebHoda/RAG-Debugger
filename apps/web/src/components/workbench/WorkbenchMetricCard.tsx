import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import styles from "./WorkbenchMetricCard.module.css";
import type { WorkbenchStatusTone } from "./WorkbenchStatusPill";

type WorkbenchMetricTone = WorkbenchStatusTone | "good" | "critical";

interface WorkbenchMetricCardProps {
  className?: string;
  detail?: ReactNode;
  icon?: LucideIcon;
  label: ReactNode;
  tone?: WorkbenchMetricTone;
  value: ReactNode;
}

export function WorkbenchMetricCard({
  className,
  detail,
  icon: Icon,
  label,
  tone = "neutral",
  value,
}: WorkbenchMetricCardProps) {
  return (
    <article
      className={[styles.card, styles[tone], className]
        .filter(Boolean)
        .join(" ")}
      data-workbench-metric-card
    >
      <div className={styles.label}>
        {Icon ? <Icon aria-hidden="true" size={17} /> : null}
        <span>{label}</span>
      </div>
      <strong>{value}</strong>
      {detail ? <small>{detail}</small> : null}
    </article>
  );
}
