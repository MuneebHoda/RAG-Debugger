import { domAnimation, LazyMotion, MotionConfig } from "motion/react";

import {
  DiagnosticProofBand,
  DiagnosticStorySection,
} from "./DiagnosticStorySection";
import { HeroCommandCenter } from "./HeroCommandCenter";
import { CiGateSection, QualityLoopSection } from "./QualityAndCiSections";
import {
  AuditReportSection,
  LandingCtaSection,
  LocalFirstTrustSection,
} from "./ReportTrustCtaSections";
import styles from "./LandingPage.module.css";

export function LandingPage() {
  return (
    <LazyMotion features={domAnimation} strict>
      <MotionConfig reducedMotion="user">
        <main className={styles.page}>
          <HeroCommandCenter />
          <DiagnosticProofBand />
          <DiagnosticStorySection />
          <QualityLoopSection />
          <CiGateSection />
          <AuditReportSection />
          <LocalFirstTrustSection />
          <LandingCtaSection />
        </main>
      </MotionConfig>
    </LazyMotion>
  );
}

export default LandingPage;
