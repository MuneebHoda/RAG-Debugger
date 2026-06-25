import styles from "./CorpusLabLogo.module.css";

type CorpusLabLogoProps = {
  tone?: "dark" | "light";
  compact?: boolean;
};

export function CorpusLabLogo({
  tone = "dark",
  compact = false,
}: CorpusLabLogoProps) {
  return (
    <span className={`${styles.logo} ${styles[tone]}`} aria-label="CorpusLab">
      <span className={styles.mark} aria-hidden="true">
        <svg viewBox="0 0 42 42" role="img">
          <path d="M21 4 36 12.5v17L21 38 6 29.5v-17L21 4Z" />
          <path d="M13.5 16.5 21 12l7.5 4.5v8.8L21 30l-7.5-4.7v-8.8Z" />
          <path d="M21 12v18M13.5 16.5 21 21l7.5-4.5" />
        </svg>
      </span>
      {compact ? null : <strong>CorpusLab</strong>}
    </span>
  );
}
