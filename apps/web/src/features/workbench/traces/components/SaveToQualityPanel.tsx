import { SaveEvidenceToEvalPanel } from "../../eval-lab/evidence/SaveEvidenceToEvalPanel";
import type { Trace } from "../../../../lib/api/traces";
import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import styles from "../TraceDetailPage.module.css";

export function SaveToQualityPanel({ trace }: { trace: Trace }) {
  if (trace.ingestion?.privacy_mode === "metadata_only" || !trace.input) {
    return (
      <WorkbenchPanel
        className={styles.panel}
        title="Add to Quality"
        description="Create a reproducible Eval Lab case from this failure."
      >
        <p>
          The query was not retained, so this imported trace cannot become an
          Eval Lab case. Re-ingest it with snippets allowed under the project
          policy.
        </p>
      </WorkbenchPanel>
    );
  }
  const hits = trace.retrieval?.hits ?? [];
  return (
    <SaveEvidenceToEvalPanel
      candidateHits={hits.map((hit) => ({
        chunkId: hit.chunk.id,
        documentId: hit.document.id,
        label: hit.citation.label,
        rank: hit.rank,
        path: hit.document.path,
        sectionTitle: hit.chunk.section_title,
        snippet: hit.snippet,
        weak: hit.evidence_strength === "weak",
        duplicate: hit.duplicate_count > 1,
      }))}
      query={trace.input}
      sourceIdentity={trace.id}
      sourceNote={`Saved from trace ${trace.id.slice(0, 8)}.`}
      topK={trace.retrieval?.run.top_k ?? 5}
    />
  );
}
