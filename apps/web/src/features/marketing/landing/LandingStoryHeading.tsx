import type { ReactNode } from "react";

import styles from "./LandingStoryHeading.module.css";

type LandingStoryHeadingProps = {
  eyebrow: string;
  title: string;
  description: string;
  titleId: string;
  tone?: "light" | "dark";
  className?: string;
  children?: ReactNode;
};

export function LandingStoryHeading({
  eyebrow,
  title,
  description,
  titleId,
  tone = "light",
  className,
  children,
}: LandingStoryHeadingProps) {
  return (
    <header
      className={[styles.heading, styles[tone], className]
        .filter(Boolean)
        .join(" ")}
    >
      <p>{eyebrow}</p>
      <h2 id={titleId}>{title}</h2>
      <span>{description}</span>
      {children}
    </header>
  );
}
