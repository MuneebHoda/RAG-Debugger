import { ArrowRight, PlayCircle } from "lucide-react";
import { Link } from "react-router-dom";

import type { OverviewAction } from "../../lib/apiClient";
import styles from "./ActionCard.module.css";

export function ActionCard({ action }: { action: OverviewAction }) {
  return (
    <Link
      className={`${styles.card} ${styles[action.priority]}`}
      to={action.route}
    >
      <PlayCircle aria-hidden="true" size={18} />
      <span>
        <strong>{action.label}</strong>
        <small>{action.detail}</small>
      </span>
      <ArrowRight aria-hidden="true" size={16} />
    </Link>
  );
}
