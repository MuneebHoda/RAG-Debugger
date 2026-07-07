import type { LucideIcon } from "lucide-react";
import type { HTMLAttributes, ReactNode } from "react";

import styles from "./WorkbenchStatusPill.module.css";

export type WorkbenchStatusTone =
  | "neutral"
  | "info"
  | "success"
  | "warning"
  | "danger"
  | "accent";

interface WorkbenchStatusPillProps extends Omit<
  HTMLAttributes<HTMLSpanElement>,
  "children"
> {
  children: ReactNode;
  icon?: LucideIcon;
  tone?: WorkbenchStatusTone;
}

export function WorkbenchStatusPill({
  children,
  className,
  icon: Icon,
  tone = "neutral",
  ...rest
}: WorkbenchStatusPillProps) {
  return (
    <span
      {...rest}
      className={[styles.pill, styles[tone], className]
        .filter(Boolean)
        .join(" ")}
      data-workbench-status-pill
      data-tone={tone}
    >
      {Icon ? <Icon aria-hidden="true" size={13} /> : null}
      {children}
    </span>
  );
}
