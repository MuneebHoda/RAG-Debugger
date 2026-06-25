import {
  ArrowRight,
  CheckCircle2,
  CircleAlert,
  CircleDashed,
  Clock3,
} from "lucide-react";
import { Link } from "react-router-dom";

import type { OverviewPipelineStep } from "../../lib/api/overview";
import styles from "./PipelineStep.module.css";

const statusIcon = {
  complete: CheckCircle2,
  warning: CircleAlert,
  pending: Clock3,
  blocked: CircleDashed,
};

export function PipelineStep({ step }: { step: OverviewPipelineStep }) {
  const Icon = statusIcon[step.status];

  return (
    <article className={`${styles.step} ${styles[step.status]}`}>
      <div className={styles.top}>
        <Icon aria-hidden="true" size={17} />
        <strong>{step.label}</strong>
      </div>
      <span className={styles.count}>{step.count}</span>
      <small>{step.detail}</small>
      <Link to={step.route}>
        {step.action_label}
        <ArrowRight aria-hidden="true" size={14} />
      </Link>
    </article>
  );
}
