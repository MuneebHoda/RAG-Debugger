import { ArrowRight } from "lucide-react";
import { m, useReducedMotion } from "motion/react";

import { ButtonLink } from "../../../components/ui/Button";
import { capabilityGroups, enterpriseSignals, outcomes } from "./landingData";
import { revealVariants, viewportOnce } from "./motion";
import styles from "./EditorialSections.module.css";

export function OutcomeRail() {
  return (
    <section className={styles.outcomes} aria-label="CorpusLab outcomes">
      {outcomes.map(([value, label]) => (
        <div key={label}>
          <strong>{value}</strong>
          <span>{label}</span>
        </div>
      ))}
    </section>
  );
}

export function CapabilityStory() {
  const reducedMotion = useReducedMotion();

  return (
    <m.section
      className={styles.capabilities}
      initial={reducedMotion ? false : "hidden"}
      variants={revealVariants}
      viewport={viewportOnce}
      whileInView="visible"
      aria-labelledby="capability-title"
    >
      <header className={styles.capabilityHeading}>
        <p>The complete quality loop</p>
        <h2 id="capability-title">Build, test, debug, measure, and share.</h2>
        <span>
          CorpusLab keeps every quality decision connected to the evidence that
          produced it.
        </span>
      </header>
      <div className={styles.capabilityRows}>
        {capabilityGroups.map((group, index) => (
          <article className={styles.capabilityRow} key={group.label}>
            <span className={styles.index}>
              {String(index + 1).padStart(2, "0")}
            </span>
            <group.icon aria-hidden="true" size={20} />
            <div>
              <p>{group.label}</p>
              <h3>{group.title}</h3>
            </div>
            <p className={styles.description}>{group.description}</p>
            <ul>
              {group.signals.map((signal) => (
                <li key={signal}>{signal}</li>
              ))}
            </ul>
          </article>
        ))}
      </div>
    </m.section>
  );
}

export function EnterpriseBand() {
  return (
    <section className={styles.enterprise} aria-labelledby="enterprise-title">
      <div className={styles.enterpriseInner}>
        <div>
          <p>Built for serious RAG systems</p>
          <h2 id="enterprise-title">
            Private when required. Collaborative when useful.
          </h2>
        </div>
        <div className={styles.enterpriseSignals}>
          {enterpriseSignals.map((signal) => (
            <span key={signal.label}>
              <signal.icon aria-hidden="true" size={18} /> {signal.label}
            </span>
          ))}
        </div>
      </div>
    </section>
  );
}

export function FinalCta() {
  return (
    <section className={styles.finalCta}>
      <div>
        <p>Evidence should survive scrutiny.</p>
        <h2>Ship RAG answers your team can defend.</h2>
      </div>
      <ButtonLink to="/signup">
        Create your workspace <ArrowRight aria-hidden="true" size={17} />
      </ButtonLink>
    </section>
  );
}
