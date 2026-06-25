import { Link, Outlet } from "react-router-dom";

import { CorpusLabLogo } from "../components/brand/CorpusLabLogo";
import styles from "./AuthLayout.module.css";

export function AuthLayout() {
  return (
    <div className={styles.layout}>
      <Link to="/" className={styles.brand} aria-label="CorpusLab home">
        <CorpusLabLogo tone="light" />
      </Link>
      <main className={styles.card}>
        <Outlet />
      </main>
      <aside className={styles.story}>
        <strong>CorpusLab turns every answer into inspectable evidence.</strong>
        <span>
          Teams use one workspace for ingestion, retrieval comparison, evals,
          reports, and production RAG observability.
        </span>
      </aside>
    </div>
  );
}
