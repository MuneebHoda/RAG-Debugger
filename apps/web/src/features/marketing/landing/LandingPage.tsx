import { domAnimation, LazyMotion, MotionConfig } from "motion/react";

import {
  CapabilityStory,
  EnterpriseBand,
  FinalCta,
  OutcomeRail,
} from "./EditorialSections";
import { FailureStory } from "./FailureStory";
import { HeroCommandCenter } from "./HeroCommandCenter";
import { ProductTour } from "./ProductTour";
import { RetrievalDemo } from "./RetrievalDemo";
import styles from "./LandingPage.module.css";

export function LandingPage() {
  return (
    <LazyMotion features={domAnimation} strict>
      <MotionConfig reducedMotion="user">
        <main className={styles.page}>
          <HeroCommandCenter />
          <OutcomeRail />
          <FailureStory />
          <RetrievalDemo />
          <CapabilityStory />
          <ProductTour />
          <EnterpriseBand />
          <FinalCta />
        </main>
      </MotionConfig>
    </LazyMotion>
  );
}

export default LandingPage;
