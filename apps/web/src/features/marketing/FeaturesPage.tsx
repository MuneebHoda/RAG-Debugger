import { FeatureCard } from "../../components/ui/FeatureCard";
import { ProductMockup } from "../../components/ui/ProductMockup";
import {
  platformFeatures,
  primaryFeatures,
  productImages,
} from "./marketingData";
import styles from "./FeaturesPage.module.css";

export function FeaturesPage() {
  return (
    <main className={styles.page}>
      <section className={styles.hero}>
        <p className={styles.eyebrow}>Complete RAG operations</p>
        <h1>Every feature your retrieval team needs in one evidence layer.</h1>
        <p>
          CorpusLab connects ingestion, chunking, embeddings, retrieval, evals,
          traces, reports, teams, privacy, and large-corpus operations so
          quality is visible from source document to production answer.
        </p>
      </section>

      <section className={styles.imageGrid} aria-label="Product screenshots">
        <ProductMockup
          src={productImages.sources}
          alt="CorpusLab Sources page with document profiles and chunk quality flags"
          label="Corpus intelligence"
        />
        <ProductMockup
          src={productImages.evals}
          alt="CorpusLab Evals page with pass rate, recall, precision, and mode comparison"
          label="Eval quality"
        />
      </section>

      <section className={styles.featureGrid} aria-label="Core feature set">
        {primaryFeatures.map((feature) => (
          <FeatureCard
            key={feature.title}
            icon={feature.icon}
            title={feature.title}
          >
            {feature.description}
          </FeatureCard>
        ))}
      </section>

      <section className={styles.deepDive}>
        <div>
          <p className={styles.eyebrow}>Platform depth</p>
          <h2>CorpusLab serves engineering, product, and review teams.</h2>
        </div>
        <ProductMockup
          src={productImages.reports}
          alt="CorpusLab Reports page with failed-query diagnosis and export-ready evidence"
          label="Shareable retrieval reports"
        />
      </section>

      <section className={styles.platformGrid} aria-label="Platform features">
        {platformFeatures.map((feature) => (
          <FeatureCard
            key={feature.title}
            icon={feature.icon}
            title={feature.title}
          >
            {feature.description}
          </FeatureCard>
        ))}
      </section>
    </main>
  );
}
