import { Activity, ArrowRight } from "lucide-react";
import {
  m,
  useMotionValue,
  useReducedMotion,
  useSpring,
  useTransform,
} from "motion/react";
import { type PointerEvent, useRef } from "react";

import { ButtonLink } from "../../../components/ui/Button";
import { EvidenceReactor } from "./EvidenceReactor";
import { HeroCommandCenterSurface } from "./HeroCommandCenterSurface";
import { motionTransition } from "./motion";
import { useCommandCenterSimulation } from "./useCommandCenterSimulation";
import styles from "./HeroCommandCenter.module.css";

export function HeroCommandCenter() {
  const sectionRef = useRef<HTMLElement>(null);
  const reducedMotion = useReducedMotion();
  const pointerX = useMotionValue(0);
  const pointerY = useMotionValue(0);
  const glowX = useSpring(pointerX, { stiffness: 110, damping: 30 });
  const glowY = useSpring(pointerY, { stiffness: 110, damping: 30 });
  const reactorX = useTransform(glowX, [-260, 1_100], [-6, 6]);
  const reactorY = useTransform(glowY, [-260, 900], [-4, 4]);
  const { activeScenario, isPlaying, selectScenario, togglePlayback } =
    useCommandCenterSimulation();

  function handlePointerMove(event: PointerEvent<HTMLElement>) {
    if (reducedMotion) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    pointerX.set(event.clientX - bounds.left - 260);
    pointerY.set(event.clientY - bounds.top - 260);
  }

  return (
    <section
      className={styles.hero}
      data-landing-header-tone="hero"
      ref={sectionRef}
      onPointerMove={handlePointerMove}
    >
      <m.div
        aria-hidden="true"
        className={styles.glow}
        style={{ x: reducedMotion ? 0 : glowX, y: reducedMotion ? 0 : glowY }}
      />

      <div className={styles.inner}>
        <m.header
          animate={{ opacity: 1, y: 0 }}
          className={styles.copy}
          initial={{ opacity: 0, y: reducedMotion ? 0 : 18 }}
          transition={reducedMotion ? { duration: 0 } : motionTransition}
        >
          <p className={styles.eyebrow}>
            <Activity aria-hidden="true" size={14} /> RAG quality command center
          </p>
          <h1>See why your RAG answer failed.</h1>
          <p className={styles.lede}>
            CorpusLab turns retrieval runs into evidence maps, failure labels,
            score explanations, eval gates, and audit reports — so teams can
            ship document AI with confidence.
          </p>
          <div className={styles.actions}>
            <ButtonLink to="/app">
              Run the guided demo <ArrowRight aria-hidden="true" size={17} />
            </ButtonLink>
            <ButtonLink to="/app/traces" variant="secondary">
              View the debugger
            </ButtonLink>
          </div>
        </m.header>

        <div className={styles.stage}>
          <EvidenceReactor
            parallaxX={reactorX}
            parallaxY={reactorY}
            scenario={activeScenario}
          />
          <div className={styles.stageFrame}>
            <HeroCommandCenterSurface
              isPlaying={isPlaying}
              scenario={activeScenario}
              onSelectScenario={selectScenario}
              onTogglePlayback={togglePlayback}
            />
          </div>
        </div>
      </div>
    </section>
  );
}
