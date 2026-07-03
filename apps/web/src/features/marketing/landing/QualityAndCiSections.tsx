import {
  ArrowDownRight,
  CheckCircle2,
  CircleX,
  GitPullRequest,
} from "lucide-react";
import { m, useReducedMotion } from "motion/react";

import { experimentResults, qualityLoopSteps } from "./landingStoryData";
import { LandingStoryHeading } from "./LandingStoryHeading";
import { revealVariants, viewportOnce } from "./motion";
import styles from "./QualityAndCiSections.module.css";

export function QualityLoopSection() {
  const reducedMotion = useReducedMotion();

  return (
    <m.section
      className={styles.quality}
      data-landing-header-tone="light"
      id="quality-loop"
      initial={reducedMotion ? false : "hidden"}
      variants={revealVariants}
      viewport={viewportOnce}
      whileInView="visible"
      aria-labelledby="quality-loop-title"
    >
      <LandingStoryHeading
        description="Preserve the evidence you expected, compare retrieval behavior, and make the release decision repeatable."
        eyebrow="Eval Lab"
        title="Turn a bad run into a regression test."
        titleId="quality-loop-title"
      />

      <ol className={styles.loop} aria-label="Trace to quality gate workflow">
        {qualityLoopSteps.map((step) => (
          <li key={step.index}>
            <span>{step.index}</span>
            <p>{step.label}</p>
            <h3>{step.title}</h3>
            <small>{step.detail}</small>
          </li>
        ))}
      </ol>

      <div className={styles.experiment}>
        <div className={styles.experimentHeading}>
          <span>
            <small>Example experiment</small>
            <strong>account-recovery-critical · top_k 5</strong>
          </span>
          <b>Gate threshold: recall@5 ≥ 0.80</b>
        </div>
        <div className={styles.tableWrap}>
          <table>
            <thead>
              <tr>
                <th>Mode</th>
                <th>Recall@5</th>
                <th>Precision@5</th>
                <th>MRR</th>
                <th>Latency</th>
                <th>Gate</th>
              </tr>
            </thead>
            <tbody>
              {experimentResults.map((result) => (
                <tr key={result.mode}>
                  <th>{result.mode}</th>
                  <td>{result.recall}</td>
                  <td>{result.precision}</td>
                  <td>{result.mrr}</td>
                  <td>{result.latency}</td>
                  <td>
                    <span
                      className={
                        result.status === "Pass" ? styles.pass : styles.fail
                      }
                    >
                      {result.status === "Pass" ? (
                        <CheckCircle2 aria-hidden="true" size={13} />
                      ) : (
                        <CircleX aria-hidden="true" size={13} />
                      )}
                      {result.status}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </m.section>
  );
}

export function CiGateSection() {
  const reducedMotion = useReducedMotion();

  return (
    <m.section
      className={styles.ci}
      data-landing-header-tone="dark"
      id="ci-gate"
      initial={reducedMotion ? false : "hidden"}
      variants={revealVariants}
      viewport={viewportOnce}
      whileInView="visible"
      aria-labelledby="ci-gate-title"
    >
      <div className={styles.ciInner}>
        <LandingStoryHeading
          description="CorpusLab runs the saved dataset against the proposed retrieval configuration and returns a deterministic merge decision."
          eyebrow="CI quality gate"
          title="Fail the PR before weak evidence ships."
          titleId="ci-gate-title"
          tone="dark"
        />

        <div className={styles.ciRun} aria-label="Failed CI eval gate example">
          <div className={styles.ciRunHeader}>
            <GitPullRequest aria-hidden="true" size={18} />
            <span>
              <small>pull request #184 · chunking/rewrite</small>
              <strong>CorpusLab Eval Gate</strong>
            </span>
            <b>
              <CircleX aria-hidden="true" size={15} /> Failed
            </b>
          </div>
          <div className={styles.deltas}>
            <div>
              <span>Recall@5</span>
              <strong>
                <ArrowDownRight aria-hidden="true" size={16} /> -0.13
              </strong>
            </div>
            <div>
              <span>MRR</span>
              <strong>
                <ArrowDownRight aria-hidden="true" size={16} /> -0.25
              </strong>
            </div>
            <div>
              <span>New failures</span>
              <strong>2 cases</strong>
            </div>
          </div>
          <div className={styles.failedCases}>
            <p>Newly failing cases</p>
            <span>
              <code>account_recovery</code> expected evidence fell to rank 4
            </span>
            <span>
              <code>identity_verification</code> answerability became
              insufficient
            </span>
          </div>
          <div className={styles.ciDecision}>
            <CircleX aria-hidden="true" size={17} />
            <span>
              <small>Merge blocked</small>
              <strong>
                Restore recall or update the expected evidence with review.
              </strong>
            </span>
          </div>
        </div>
      </div>
    </m.section>
  );
}
