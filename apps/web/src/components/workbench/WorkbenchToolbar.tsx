import type { HTMLAttributes, ReactNode } from "react";

import styles from "./WorkbenchToolbar.module.css";

interface WorkbenchToolbarProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  label: string;
}

export function WorkbenchToolbar({
  children,
  className,
  label,
  ...rest
}: WorkbenchToolbarProps) {
  return (
    <div
      {...rest}
      aria-label={label}
      className={[styles.toolbar, className].filter(Boolean).join(" ")}
      data-workbench-toolbar
    >
      {children}
    </div>
  );
}
