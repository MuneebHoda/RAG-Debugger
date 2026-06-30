import type { RetrievalQueryHit } from "../../../../lib/api/retrieval";
import type { Trace } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";
import { TraceScoreBars } from "./TraceMetrics";

export function TraceEvidencePanel({ trace }: { trace: Trace }) {
  const hits = trace.retrieval?.hits ?? [];
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Ranked evidence</h2>
          <p>{hits.length} chunks were returned for this run.</p>
        </div>
      </div>
      {hits.length === 0 ? (
        <p className={styles.answer}>No evidence was retrieved.</p>
      ) : (
        <div className={styles.evidenceList}>
          {hits.map((hit) => {
            const explanation = trace.diagnosis?.score_explanations.find(
              (item) => item.chunk_id === hit.chunk.id,
            );
            return (
              <EvidenceCard
                explanation={explanation}
                hit={hit}
                key={hit.chunk.id}
              />
            );
          })}
        </div>
      )}
    </section>
  );
}

function EvidenceCard({
  hit,
  explanation,
}: {
  hit: RetrievalQueryHit;
  explanation?: NonNullable<Trace["diagnosis"]>["score_explanations"][number];
}) {
  return (
    <article className={styles.evidenceCard}>
      <div className={styles.evidenceHeader}>
        <strong>
          #{hit.rank} {hit.document.path}
        </strong>
        <span className={styles[hit.evidence_strength]}>
          {hit.evidence_strength}
        </span>
      </div>
      <p>{hit.snippet}</p>
      <div className={styles.metadata}>
        <span>score {hit.score.toFixed(2)}</span>
        <span>chunk {hit.chunk.ordinal + 1}</span>
        {hit.chunk.section_title ? (
          <span>{hit.chunk.section_title}</span>
        ) : null}
        <span>{hit.citation.checksum_prefix}</span>
      </div>
      <TraceScoreBars explanation={explanation} hit={hit} />
    </article>
  );
}
