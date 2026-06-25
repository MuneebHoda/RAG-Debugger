import { ArrowRight, CircleAlert, Info, ShieldAlert } from "lucide-react";
import { Link } from "react-router-dom";

import type { OverviewIssue } from "../../lib/api/overview";
import styles from "./RiskList.module.css";

const severityIcon = {
  info: Info,
  warning: CircleAlert,
  critical: ShieldAlert,
};

export function RiskList({ issues }: { issues: OverviewIssue[] }) {
  return (
    <section className={styles.panel} aria-labelledby="risk-center-title">
      <div className={styles.heading}>
        <div>
          <p>Risk center</p>
          <h2 id="risk-center-title">What needs attention</h2>
        </div>
        <span>{issues.length} open</span>
      </div>
      <div className={styles.list}>
        {issues.length === 0 ? (
          <div className={styles.empty}>No active corpus risks detected.</div>
        ) : (
          issues.map((issue) => {
            const Icon = severityIcon[issue.severity];
            return (
              <article
                key={issue.id}
                className={`${styles.issue} ${styles[issue.severity]}`}
              >
                <Icon aria-hidden="true" size={17} />
                <div>
                  <strong>{issue.title}</strong>
                  <p>{issue.detail}</p>
                </div>
                <Link to={issue.route}>
                  {issue.action_label}
                  <ArrowRight aria-hidden="true" size={14} />
                </Link>
              </article>
            );
          })
        )}
      </div>
    </section>
  );
}
