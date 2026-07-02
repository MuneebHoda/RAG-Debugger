import { AnimatePresence, m, useReducedMotion } from "motion/react";

import { quickTransition } from "./motion";
import styles from "./AnimatedMetric.module.css";

type AnimatedMetricProps = {
  label: string;
  value: string;
  tone?: "neutral" | "positive" | "warning" | "critical";
};

export function AnimatedMetric({
  label,
  value,
  tone = "neutral",
}: AnimatedMetricProps) {
  const reducedMotion = useReducedMotion();

  return (
    <div className={styles.metric}>
      <span>{label}</span>
      <AnimatePresence initial={false} mode="wait">
        <m.strong
          animate={{ opacity: 1, y: 0 }}
          className={styles[tone]}
          exit={{ opacity: 0, y: reducedMotion ? 0 : -4 }}
          initial={{ opacity: 0, y: reducedMotion ? 0 : 4 }}
          key={value}
          transition={reducedMotion ? { duration: 0 } : quickTransition}
        >
          {value}
        </m.strong>
      </AnimatePresence>
    </div>
  );
}
