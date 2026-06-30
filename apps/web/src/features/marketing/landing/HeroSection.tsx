import { ArrowRight, CircleCheck, Cpu, ShieldCheck } from "lucide-react";
import {
  m,
  useMotionValue,
  useReducedMotion,
  useScroll,
  useSpring,
  useTransform,
} from "motion/react";
import { type PointerEvent, useRef } from "react";

import { ButtonLink } from "../../../components/ui/Button";
import { themeImages } from "../marketingData";
import { motionTransition } from "./motion";
import styles from "./HeroSection.module.css";

export function HeroSection() {
  const sectionRef = useRef<HTMLElement>(null);
  const reducedMotion = useReducedMotion();
  const pointerX = useMotionValue(0);
  const pointerY = useMotionValue(0);
  const sceneX = useSpring(pointerX, { stiffness: 90, damping: 24 });
  const sceneY = useSpring(pointerY, { stiffness: 90, damping: 24 });
  const { scrollYProgress } = useScroll({
    target: sectionRef,
    offset: ["start start", "end start"],
  });
  const scrollOffset = useTransform(scrollYProgress, [0, 1], [0, 38]);

  function handlePointerMove(event: PointerEvent<HTMLElement>) {
    if (reducedMotion) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    pointerX.set(((event.clientX - bounds.left) / bounds.width - 0.5) * 16);
    pointerY.set(((event.clientY - bounds.top) / bounds.height - 0.5) * 12);
  }

  function resetPointer() {
    pointerX.set(0);
    pointerY.set(0);
  }

  return (
    <section
      className={styles.hero}
      ref={sectionRef}
      onPointerLeave={resetPointer}
      onPointerMove={handlePointerMove}
    >
      <m.div
        aria-hidden="true"
        className={styles.scene}
        style={{ y: reducedMotion ? 0 : scrollOffset }}
      >
        <m.img
          alt=""
          fetchPriority="high"
          src={themeImages.hero}
          style={{
            x: reducedMotion ? 0 : sceneX,
            y: reducedMotion ? 0 : sceneY,
          }}
        />
      </m.div>
      <div aria-hidden="true" className={styles.scrim} />

      <m.div
        animate="visible"
        className={styles.copy}
        initial="hidden"
        variants={{
          hidden: { opacity: 0, y: reducedMotion ? 0 : 20 },
          visible: {
            opacity: 1,
            y: 0,
            transition: reducedMotion
              ? { duration: 0 }
              : { ...motionTransition, staggerChildren: 0.07 },
          },
        }}
      >
        <m.p className={styles.eyebrow} variants={heroChild(reducedMotion)}>
          Corpus intelligence for evidence-first teams
        </m.p>
        <m.h1 variants={heroChild(reducedMotion)}>
          Make every RAG answer defensible.
        </m.h1>
        <m.p className={styles.lede} variants={heroChild(reducedMotion)}>
          CorpusLab reveals exactly where retrieval breaks, why evidence ranks,
          and whether a change is safe to ship, from source document to cited
          answer.
        </m.p>
        <m.div className={styles.actions} variants={heroChild(reducedMotion)}>
          <ButtonLink to="/signup">
            Start debugging <ArrowRight aria-hidden="true" size={17} />
          </ButtonLink>
          <ButtonLink to="/features" variant="ghost">
            Explore the platform
          </ButtonLink>
        </m.div>
        <m.div
          aria-label="Platform capabilities"
          className={styles.capabilityRail}
          variants={heroChild(reducedMotion)}
        >
          <span>
            <ShieldCheck aria-hidden="true" size={15} /> Private corpora
          </span>
          <span>
            <CircleCheck aria-hidden="true" size={15} /> Cited evidence
          </span>
          <span>
            <Cpu aria-hidden="true" size={15} /> GPU-ready workers
          </span>
        </m.div>
      </m.div>
    </section>
  );
}

function heroChild(reducedMotion: boolean | null) {
  return {
    hidden: { opacity: 0, y: reducedMotion ? 0 : 14 },
    visible: {
      opacity: 1,
      y: 0,
      transition: reducedMotion ? { duration: 0 } : motionTransition,
    },
  };
}
