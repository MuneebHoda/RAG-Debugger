import type { LucideIcon } from "lucide-react";
import type { HTMLAttributes, ReactNode } from "react";
import { useId } from "react";

import styles from "./WorkbenchPanel.module.css";

type WorkbenchPanelDensity = "default" | "compact";
type WorkbenchPanelTone = "default" | "accent" | "warning" | "danger";

interface WorkbenchPanelProps extends Omit<
  HTMLAttributes<HTMLElement>,
  "title"
> {
  actions?: ReactNode;
  children: ReactNode;
  contentClassName?: string;
  density?: WorkbenchPanelDensity;
  description?: ReactNode;
  eyebrow?: ReactNode;
  icon?: LucideIcon;
  title?: ReactNode;
  titleId?: string;
  tone?: WorkbenchPanelTone;
}

export function WorkbenchPanel({
  actions,
  children,
  className,
  contentClassName,
  density = "default",
  description,
  eyebrow,
  icon: Icon,
  title,
  titleId,
  tone = "default",
  ...rest
}: WorkbenchPanelProps) {
  const generatedId = useId();
  const headingId = titleId ?? generatedId;
  const hasHeader = Boolean(title || description || eyebrow || actions || Icon);
  const labelledBy = title ? headingId : rest["aria-labelledby"];

  return (
    <section
      {...rest}
      aria-labelledby={labelledBy}
      className={[styles.panel, styles[density], styles[tone], className]
        .filter(Boolean)
        .join(" ")}
      data-workbench-panel
    >
      {hasHeader ? (
        <header className={styles.header}>
          <div className={styles.heading}>
            {Icon ? (
              <span className={styles.icon}>
                <Icon aria-hidden="true" size={17} />
              </span>
            ) : null}
            <div className={styles.copy}>
              {eyebrow ? <p className={styles.eyebrow}>{eyebrow}</p> : null}
              {title ? <h2 id={headingId}>{title}</h2> : null}
              {description ? (
                <p className={styles.description}>{description}</p>
              ) : null}
            </div>
          </div>
          {actions ? <div className={styles.actions}>{actions}</div> : null}
        </header>
      ) : null}
      <div
        className={[styles.content, contentClassName].filter(Boolean).join(" ")}
      >
        {children}
      </div>
    </section>
  );
}
