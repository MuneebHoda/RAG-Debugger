import { useState } from "react";

import { PricingCard } from "../../components/ui/PricingCard";
import { pricingTiers } from "./marketingData";
import styles from "./PricingPage.module.css";

export function PricingPage() {
  const [billing, setBilling] = useState<"monthly" | "annual">("monthly");

  return (
    <main className={styles.page}>
      <section className={styles.hero}>
        <p className={styles.eyebrow}>Subscription plus usage</p>
        <h1>Pricing that starts simple and scales fairly.</h1>
        <p>
          CorpusLab uses predictable workspace subscriptions with included
          platform units for traces, eval scores, report generation, chunk
          indexing, and embedding work.
        </p>
        <div className={styles.toggle} aria-label="Billing period">
          <button
            className={billing === "monthly" ? styles.active : ""}
            type="button"
            onClick={() => setBilling("monthly")}
          >
            Monthly
          </button>
          <button
            className={billing === "annual" ? styles.active : ""}
            type="button"
            onClick={() => setBilling("annual")}
          >
            Annual · save 16%
          </button>
        </div>
      </section>

      <section className={styles.grid} aria-label="Pricing tiers">
        {pricingTiers.map((tier) => (
          <PricingCard key={tier.name} tier={tier} />
        ))}
      </section>

      <section className={styles.units}>
        <h2>How platform units work</h2>
        <p>
          Platform units combine the operational work CorpusLab performs across
          traces, eval scores, report generation, chunk indexing, embeddings,
          and indexing jobs. Teams get predictable base capacity and fair
          overage pricing as corpora and traffic grow.
        </p>
      </section>
    </main>
  );
}
