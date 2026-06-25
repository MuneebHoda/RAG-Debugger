import {
  ArrowRight,
  BadgeCheck,
  BarChart3,
  Layers3,
  ShieldCheck,
} from "lucide-react";

import { ButtonLink } from "../../components/ui/Button";
import { FeatureCard } from "../../components/ui/FeatureCard";
import { ProductMockup } from "../../components/ui/ProductMockup";
import {
  heroStats,
  primaryFeatures,
  productImages,
  themeImages,
  trustSignals,
  workflowSteps,
} from "./marketingData";
import styles from "./LandingPage.module.css";

export function LandingPage() {
  return (
    <main>
      <section className={styles.hero}>
        <img
          className={styles.heroImage}
          src={themeImages.hero}
          alt="Abstract CorpusLab evidence intelligence map with document nodes, citation paths, and evaluation signals"
        />
        <div className={styles.heroCopy}>
          <div className={styles.kicker}>
            <BadgeCheck aria-hidden="true" size={16} />
            RAG observability for evidence-first teams
          </div>
          <h1>Turn every corpus into trusted retrieval.</h1>
          <p>
            CorpusLab helps RAG teams ingest messy knowledge, inspect chunk and
            embedding quality, compare retrieval modes, prove citations with
            evals, and ship answers backed by evidence.
          </p>
          <div className={styles.ctas}>
            <ButtonLink to="/signup">
              Start debugging <ArrowRight aria-hidden="true" size={17} />
            </ButtonLink>
            <ButtonLink to="/features" variant="ghost">
              View platform tour
            </ButtonLink>
          </div>
          <div className={styles.trustRow}>
            {trustSignals.map((signal) => (
              <span key={signal.label}>
                <signal.icon aria-hidden="true" size={15} />
                {signal.label}
              </span>
            ))}
          </div>
        </div>
      </section>

      <section className={styles.stats} aria-label="CorpusLab outcomes">
        {heroStats.map(([value, label]) => (
          <article key={label}>
            <strong>{value}</strong>
            <span>{label}</span>
          </article>
        ))}
      </section>

      <section className={styles.story}>
        <div>
          <p className={styles.eyebrow}>The operating system for RAG quality</p>
          <h2>CorpusLab shows where retrieval breaks and how to fix it.</h2>
          <p>
            Teams lose time guessing whether failures come from extraction,
            chunking, embeddings, retrieval, reranking, prompts, stale indexes,
            or missing data. CorpusLab connects every answer to source
            documents, chunks, scores, evals, and reports so quality becomes
            inspectable.
          </p>
        </div>
        <img
          className={styles.themePanel}
          src={themeImages.evidenceMap}
          alt="Abstract evidence lineage map connecting documents, chunks, citations, and eval checkpoints"
        />
      </section>

      <section className={styles.featureGrid} aria-label="Core capabilities">
        {primaryFeatures.slice(0, 6).map((feature) => (
          <FeatureCard
            key={feature.title}
            icon={feature.icon}
            title={feature.title}
          >
            {feature.description}
          </FeatureCard>
        ))}
      </section>

      <section className={styles.workflow}>
        <div>
          <p className={styles.eyebrow}>One continuous quality loop</p>
          <h2>From source document to production evidence.</h2>
        </div>
        <ol>
          {workflowSteps.map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ol>
      </section>

      <section className={styles.showcase}>
        <img
          className={styles.qualityImage}
          src={themeImages.qualityLayer}
          alt="Abstract CorpusLab quality layer showing corpus health, scoring signals, and diagnostics"
        />
        <div>
          <p className={styles.eyebrow}>Why CorpusLab wins</p>
          <h2>Evidence quality, not just vector search.</h2>
          <ul>
            <li>
              <Layers3 aria-hidden="true" size={18} />
              Debug extraction, chunking, embeddings, retrieval, prompts, and
              citations in one flow.
            </li>
            <li>
              <BarChart3 aria-hidden="true" size={18} />
              Measure quality with evals, score bars, reports, and regression
              tracking.
            </li>
            <li>
              <ShieldCheck aria-hidden="true" size={18} />
              Keep privacy, local collection, hosted teams, and enterprise
              controls in the same product model.
            </li>
          </ul>
        </div>
      </section>

      <section className={styles.productProof}>
        <div>
          <p className={styles.eyebrow}>Real workbench proof</p>
          <h2>The abstract layer is backed by an inspectable product.</h2>
          <p>
            The workbench gives teams concrete surfaces for corpus health,
            retrieval diagnosis, evals, and reports. The theme gives the brand a
            modern system language; the screenshots prove the workflow exists.
          </p>
        </div>
        <div className={styles.proofGrid}>
          <ProductMockup
            src={productImages.dashboard}
            alt="CorpusLab dashboard showing corpus health, retrieval quality, eval coverage, and reports"
            label="Corpus command center"
          />
          <ProductMockup
            src={productImages.retrieval}
            alt="CorpusLab retrieval page with evidence summary, grouped hits, score bars, and citations"
            label="Retrieval diagnosis"
          />
        </div>
      </section>

      <section className={styles.finalCta}>
        <h2>Ship RAG answers your team can defend.</h2>
        <p>
          CorpusLab gives engineers, product leaders, and compliance reviewers a
          shared language for retrieval quality.
        </p>
        <ButtonLink to="/signup">Create your CorpusLab workspace</ButtonLink>
      </section>
    </main>
  );
}
