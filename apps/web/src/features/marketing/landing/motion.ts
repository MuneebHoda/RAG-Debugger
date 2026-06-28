import type { Transition, Variants } from "motion/react";

export const motionTransition: Transition = {
  duration: 0.52,
  ease: [0.22, 1, 0.36, 1],
};

export const quickTransition: Transition = {
  duration: 0.24,
  ease: [0.22, 1, 0.36, 1],
};

export const revealVariants: Variants = {
  hidden: { opacity: 0, y: 24 },
  visible: { opacity: 1, y: 0, transition: motionTransition },
};

export const viewportOnce = {
  amount: 0.2,
  once: true,
} as const;
