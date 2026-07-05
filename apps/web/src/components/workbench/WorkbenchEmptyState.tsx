import { ArrowRight, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router-dom";

import styles from "./WorkbenchEmptyState.module.css";

interface EmptyStateAction {
  label: string;
  to?: string;
  href?: string;
  onClick?: () => void;
}

export function WorkbenchEmptyState({
  description,
  icon: Icon,
  primaryAction,
  secondaryAction,
  title,
}: {
  description: ReactNode;
  icon: LucideIcon;
  primaryAction?: EmptyStateAction;
  secondaryAction?: EmptyStateAction;
  title: string;
}) {
  return (
    <div className={styles.emptyState}>
      <span className={styles.icon}>
        <Icon aria-hidden="true" size={19} />
      </span>
      <div className={styles.copy}>
        <strong>{title}</strong>
        <p>{description}</p>
      </div>
      {primaryAction || secondaryAction ? (
        <div className={styles.actions}>
          {primaryAction ? (
            <EmptyStateAction action={primaryAction} primary />
          ) : null}
          {secondaryAction ? (
            <EmptyStateAction action={secondaryAction} />
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function EmptyStateAction({
  action,
  primary = false,
}: {
  action: EmptyStateAction;
  primary?: boolean;
}) {
  const content = (
    <>
      {action.label} <ArrowRight aria-hidden="true" size={14} />
    </>
  );
  const className = primary ? styles.primaryAction : styles.secondaryAction;

  if (action.to) {
    return (
      <Link className={className} to={action.to}>
        {content}
      </Link>
    );
  }
  if (action.href) {
    return (
      <a className={className} href={action.href}>
        {content}
      </a>
    );
  }
  return (
    <button className={className} type="button" onClick={action.onClick}>
      {content}
    </button>
  );
}
