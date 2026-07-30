import { SaveEvidenceToEvalPanel } from "../../eval-lab/evidence/SaveEvidenceToEvalPanel";
import type { Trace } from "../../../../lib/api/traces";

export function SaveToQualityPanel({ trace }: { trace: Trace }) {
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
