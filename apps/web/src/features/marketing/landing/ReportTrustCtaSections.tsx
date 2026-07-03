import {
  ArrowRight,
  FileText,
  LockKeyhole,
  Share2,
  ShieldCheck,
} from "lucide-react";
import { m, useReducedMotion } from "motion/react";

import { ButtonLink } from "../../../components/ui/Button";
import { privacyBoundary } from "./landingStoryData";
import { LandingStoryHeading } from "./LandingStoryHeading";
import { revealVariants, viewportOnce } from "./motion";
import styles from "./ReportTrustCtaSections.module.css";

export function AuditReportSection() {
  const reducedMotion = useReducedMotion();

  return (
    <m.section
      className={styles.report}
      data-landing-header-tone="light"
      id="audit-report"
      initial={reducedMotion ? false : "hidden"}
      variants={revealVariants}
      viewport={viewportOnce}
      whileInView="visible"
      aria-labelledby="audit-report-title"
    >
      <LandingStoryHeading
        className={styles.reportHeading}
        description="Freeze the failed run, evidence references, labels, and prioritized repairs in a privacy-classified engineering deliverable."
        eyebrow="RAG audit report"
        title="Share the diagnosis, not the documents."
        titleId="audit-report-title"
      >
        <ButtonLink to="/app/reports" variant="ghost">
          Open audit reports <ArrowRight aria-hidden="true" size={16} />
        </ButtonLink>
      </LandingStoryHeading>

      <article
        className={styles.reportPreview}
        aria-label="Metadata-only audit report example"
      >
        <div className={styles.reportHeader}>
          <FileText aria-hidden="true" size={18} />
          <span>
            <small>Debug report · trace source</small>
            <strong>Account recovery retrieval audit</strong>
          </span>
          <b>
            <LockKeyhole aria-hidden="true" size={13} /> metadata_only
          </b>
        </div>
        <div className={styles.reportSummary}>
          <p>Executive summary</p>
          <h3>
            Retrieval produced three candidates, but no evidence passed direct
            body-text support.
          </h3>
        </div>
        <div className={styles.reportColumns}>
          <div>
            <p>Failure labels</p>
            <code>answerability_gap</code>
            <code>semantic_only_match</code>
          </div>
          <div>
            <p>Evidence references</p>
            <span>chunk_018f...04 · rank 1 · score 0.88</span>
            <span>chunk_018f...09 · rank 2 · score 0.79</span>
          </div>
        </div>
        <div className={styles.reportAction}>
          <ArrowRight aria-hidden="true" size={16} />
          <span>
            <small>Priority 1 · corpus coverage</small>
            <strong>
              Index the expected recovery policy before changing retrieval
              weights.
            </strong>
          </span>
        </div>
      </article>
    </m.section>
  );
}

export function LocalFirstTrustSection() {
  return (
    <section
      className={styles.trust}
      data-landing-header-tone="light"
      id="local-first-trust"
      aria-labelledby="local-first-title"
    >
      <LandingStoryHeading
        description="CorpusLab separates complete local diagnostics from the metadata and reports that are safe to review elsewhere."
        eyebrow="Local-first trust boundary"
        title="Raw evidence stays local until you approve what leaves."
        titleId="local-first-title"
      />

      <div className={styles.boundary} aria-label="CorpusLab privacy boundary">
        <div className={styles.localZone}>
          <LockKeyhole aria-hidden="true" size={19} />
          <p>Local workspace</p>
          {privacyBoundary.local.map((item) => (
            <span key={item}>{item}</span>
          ))}
        </div>
        <div className={styles.approval}>
          <ShieldCheck aria-hidden="true" size={19} />
          <strong>Explicit approval</strong>
          <span>Snippets are opt-in</span>
        </div>
        <div className={styles.shareZone}>
          <Share2 aria-hidden="true" size={19} />
          <p>Approved review surface</p>
          {privacyBoundary.approved.map((item) => (
            <span key={item}>{item}</span>
          ))}
        </div>
      </div>

      <div className={styles.privacyNote}>
        <span>
          <strong>Deterministic diagnosis</strong>
          No external model call is required.
        </span>
        <span>
          <strong>Full local only</strong>
          Detailed local reports cannot be exported.
        </span>
      </div>
    </section>
  );
}

export function LandingCtaSection() {
  return (
    <section
      aria-labelledby="landing-cta-title"
      className={styles.cta}
      data-landing-header-tone="dark"
      id="landing-cta"
    >
      <div>
        <p>Follow one failure through the complete loop.</p>
        <h2 id="landing-cta-title">
          Debug the answer. Gate the fix. Share the evidence.
        </h2>
      </div>
      <div className={styles.ctaActions}>
        <ButtonLink to="/app">
          Run the guided demo <ArrowRight aria-hidden="true" size={17} />
        </ButtonLink>
        <ButtonLink to="/app/traces" variant="secondary">
          View the debugger
        </ButtonLink>
      </div>
    </section>
  );
}
