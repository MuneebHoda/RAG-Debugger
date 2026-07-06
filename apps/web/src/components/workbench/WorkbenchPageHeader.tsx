import type { ReactNode } from "react";
import { Link } from "react-router-dom";

import styles from "./WorkbenchPageHeader.module.css";

export function WorkbenchPageHeader({
  actions,
  back,
  description,
  metadata,
  section,
  title,
  titleId,
}: {
  actions?: ReactNode;
  back?: { label: string; to: string };
  description: string;
  metadata?: ReactNode;
  section: string;
  title: ReactNode;
  titleId: string;
}) {
  return (
    <header className={styles.header}>
      {back ? (
        <Link className={styles.backLink} to={back.to}>
          ← {back.label}
        </Link>
      ) : null}
      <div className={styles.mainRow}>
        <div className={styles.copy}>
          <p className={styles.eyebrow}>{section}</p>
          <h1 id={titleId}>{title}</h1>
          <p className={styles.description}>{description}</p>
        </div>
        {actions ? <div className={styles.actions}>{actions}</div> : null}
      </div>
      {metadata ? <div className={styles.metadata}>{metadata}</div> : null}
    </header>
  );
}
