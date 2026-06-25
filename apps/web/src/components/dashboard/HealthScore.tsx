import {
  ArrowRight,
  CheckCircle2,
  CircleAlert,
  CircleDashed,
} from "lucide-react";
import { Link } from "react-router-dom";

import type { OverviewHealth } from "../../lib/apiClient";
import { ProgressBar } from "./ProgressBar";
import styles from "./HealthScore.module.css";

const statusCopy: Record<OverviewHealth["status"], string> = {
  ready: "Ready",
  needs_indexing: "Needs indexing",
  needs_eval_coverage: "Needs eval coverage",
  needs_documents: "Needs documents",
};

export function HealthScore({ health }: { health: OverviewHealth }) {
  const tone =
    health.score >= 80 ? "good" : health.score >= 45 ? "warning" : "critical";
  const Icon =
    health.status === "ready"
      ? CheckCircle2
      : health.status === "needs_documents"
        ? CircleDashed
        : CircleAlert;

  return (
    <section
      className={`${styles.band} ${styles[tone]}`}
      aria-label="Corpus health"
    >
      <div className={styles.score}>
        <span>{health.score}</span>
        <small>/100</small>
      </div>
      <div className={styles.content}>
        <p className={styles.status}>
          <Icon aria-hidden="true" size={18} />
          {statusCopy[health.status]}
        </p>
        <h2>Corpus health score</h2>
        <p>{health.summary}</p>
        <ProgressBar
          value={health.score / 100}
          tone={tone}
          label="Corpus health score"
        />
      </div>
      {health.primary_action ? (
        <Link className={styles.action} to={health.primary_action.route}>
          {health.primary_action.label}
          <ArrowRight aria-hidden="true" size={16} />
        </Link>
      ) : null}
    </section>
  );
}
