import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import styles from "./FeatureCard.module.css";

type FeatureCardProps = {
  icon: LucideIcon;
  title: string;
  children: ReactNode;
};

export function FeatureCard({ icon: Icon, title, children }: FeatureCardProps) {
  return (
    <article className={styles.card}>
      <span className={styles.icon}>
        <Icon aria-hidden="true" size={20} />
      </span>
      <h3>{title}</h3>
      <p>{children}</p>
    </article>
  );
}
