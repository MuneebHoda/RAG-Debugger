import type { RetrievalQueryHit } from "../../../../lib/api/retrieval";
import type { Trace } from "../../../../lib/api/traces";
import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import { WorkbenchStatusPill } from "../../../../components/workbench/WorkbenchStatusPill";
import styles from "../TraceDetailPage.module.css";
import { TraceScoreBars } from "./TraceMetrics";

export function TraceEvidencePanel({ trace }: { trace: Trace }) {
  const hits = trace.retrieval?.hits ?? [];
  return (
    <WorkbenchPanel
      className={styles.panel}
      description={`${hits.length} chunks were returned for this run.`}
      title="Ranked evidence"
    >
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
    </WorkbenchPanel>
  );
}

function EvidenceCard({
  hit,
  explanation,
}: {
  hit: RetrievalQueryHit;
  explanation?: NonNullable<Trace["diagnosis"]>["score_explanations"][number];
}) {
  const support = hit.answer_support;
  return (
    <article className={styles.evidenceCard}>
      <div className={styles.evidenceHeader}>
        <strong>
          #{hit.rank} {hit.document.path}
        </strong>
        <WorkbenchStatusPill tone={evidenceTone(hit.evidence_strength)}>
          {hit.evidence_strength}
        </WorkbenchStatusPill>
      </div>
      <p>{hit.snippet}</p>
      <div
        className={`${styles.supportStatus} ${
          support?.status === "supported"
            ? styles.supportedEvidence
            : styles.candidateEvidence
        }`}
      >
        <strong>
          {support?.status === "supported"
            ? "Supports answer"
            : "Candidate only"}
        </strong>
        <span>{formatSupportReason(support?.reason ?? "unassessed")}</span>
      </div>
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

function evidenceTone(
  strength: RetrievalQueryHit["evidence_strength"],
): "success" | "warning" | "neutral" {
  if (strength === "strong") return "success";
  if (strength === "weak") return "warning";
  return "neutral";
}

function formatSupportReason(reason: string): string {
  return reason.replaceAll("_", " ");
}
