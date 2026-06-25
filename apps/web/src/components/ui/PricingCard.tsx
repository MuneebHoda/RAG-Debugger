import { CheckCircle2 } from "lucide-react";

import { ButtonLink } from "./Button";
import styles from "./PricingCard.module.css";

export type PricingTier = {
  name: string;
  price: string;
  description: string;
  cta: string;
  href: string;
  featured?: boolean;
  items: string[];
  usage: string;
};

export function PricingCard({ tier }: { tier: PricingTier }) {
  return (
    <article
      className={`${styles.card} ${tier.featured ? styles.featured : ""}`}
    >
      <div className={styles.header}>
        <h3>{tier.name}</h3>
        <strong>{tier.price}</strong>
        <p>{tier.description}</p>
      </div>
      <ButtonLink
        to={tier.href}
        variant={tier.featured ? "primary" : "ghost"}
        className={styles.cta}
      >
        {tier.cta}
      </ButtonLink>
      <p className={styles.usage}>{tier.usage}</p>
      <ul>
        {tier.items.map((item) => (
          <li key={item}>
            <CheckCircle2 aria-hidden="true" size={16} />
            <span>{item}</span>
          </li>
        ))}
      </ul>
    </article>
  );
}
