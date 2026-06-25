import { Database, FileText, FlaskConical, GitBranch } from "lucide-react";
import { Link } from "react-router-dom";

import type { OverviewActivity } from "../../lib/api/overview";
import styles from "./ActivityList.module.css";

const activityIcon = {
  source: Database,
  document: FileText,
  trace: GitBranch,
  eval: FlaskConical,
};

export function ActivityList({ activity }: { activity: OverviewActivity[] }) {
  return (
    <section className={styles.panel} aria-labelledby="recent-activity-title">
      <div className={styles.heading}>
        <p>Activity</p>
        <h2 id="recent-activity-title">Recent system events</h2>
      </div>
      <div className={styles.list}>
        {activity.map((item) => {
          const Icon = activityIcon[item.kind];
          return (
            <Link key={item.id} className={styles.item} to={item.route}>
              <Icon aria-hidden="true" size={17} />
              <span>
                <strong>{item.label}</strong>
                <small>{item.detail}</small>
              </span>
              {item.created_at ? (
                <time>{formatTime(item.created_at)}</time>
              ) : null}
            </Link>
          );
        })}
      </div>
    </section>
  );
}

function formatTime(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(value));
}
