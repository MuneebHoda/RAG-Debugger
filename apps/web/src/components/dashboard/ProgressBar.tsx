import styles from "./ProgressBar.module.css";

type ProgressTone = "neutral" | "good" | "warning" | "critical";

export function ProgressBar({
  value,
  tone = "neutral",
  label,
}: {
  value: number;
  tone?: ProgressTone;
  label?: string;
}) {
  const clamped = Math.max(0, Math.min(1, value));

  return (
    <div className={styles.track} aria-label={label}>
      <span
        className={`${styles.fill} ${styles[tone]}`}
        style={{ width: `${Math.round(clamped * 100)}%` }}
      />
    </div>
  );
}
