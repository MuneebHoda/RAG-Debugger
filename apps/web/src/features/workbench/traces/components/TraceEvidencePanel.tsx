import type { RetrievalQueryHit } from "../../../../lib/api/retrieval";
import type { Trace } from "../../../../lib/api/traces";
import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import { WorkbenchStatusPill } from "../../../../components/workbench/WorkbenchStatusPill";
import styles from "../TraceDetailPage.module.css";
import { TraceScoreBars } from "./TraceMetrics";

export function TraceEvidencePanel({ trace }: { trace: Trace }) {
  if (trace.ingestion) {
    return <ImportedEvidencePanel trace={trace} />;
  }
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

function ImportedEvidencePanel({ trace }: { trace: Trace }) {
  const evidence = trace.ingestion?.evidence ?? [];
  return (
    <WorkbenchPanel
      className={styles.panel}
      description={`${evidence.length} imported evidence records are permitted by this trace's privacy mode.`}
      title="Imported ranked evidence"
    >
      {evidence.length === 0 ? (
        <p className={styles.answer}>
          No retrieval evidence metadata was mapped.
        </p>
      ) : (
        <ol className={styles.evidenceList}>
          {evidence.map((item) => (
            <li className={styles.evidenceCard} key={item.external_chunk_id}>
              <div className={styles.evidenceHeader}>
                <strong>
                  #{item.rank} {item.document_label ?? item.external_chunk_id}
                </strong>
                <WorkbenchStatusPill
                  tone={
                    item.score >= 0.75
                      ? "success"
                      : item.score < 0.4
                        ? "warning"
                        : "neutral"
                  }
                >
                  score {item.score.toFixed(2)}
                </WorkbenchStatusPill>
              </div>
              <p>{item.snippet ?? "Content withheld by privacy policy."}</p>
              <div className={styles.metadata}>
                <span>ID {item.external_chunk_id}</span>
                {item.lexical_score != null ? (
                  <span>lexical {item.lexical_score.toFixed(2)}</span>
                ) : null}
                {item.semantic_score != null ? (
                  <span>semantic {item.semantic_score.toFixed(2)}</span>
                ) : null}
                {item.citation_label ? (
                  <span>citation {item.citation_label}</span>
                ) : null}
                <span>
                  answer support{" "}
                  {item.answer_support_status.replaceAll("_", " ")}
                  {item.answer_support_reason !== "unassessed"
                    ? ` · ${item.answer_support_reason.replaceAll("_", " ")}`
                    : ""}
                </span>
              </div>
            </li>
          ))}
        </ol>
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
