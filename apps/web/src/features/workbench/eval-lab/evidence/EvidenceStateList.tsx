import { useQuery } from "@tanstack/react-query";

import { queryEvalLabEvidence } from "../../../../lib/api/evalLab";
import {
  deriveEvidenceStates,
  evidenceLookupChunkIds,
  normalizeEvidenceSelection,
  type EvidenceSelection,
  type EvidenceStateContext,
} from "./evidenceSelection";
import styles from "./EvidencePicker.module.css";

export function EvidenceStateList({
  title = "Evidence states",
  selection,
  context,
  failureLabels = [],
}: {
  title?: string;
  selection: EvidenceSelection;
  context: EvidenceStateContext;
  failureLabels?: Parameters<typeof deriveEvidenceStates>[0]["failureLabels"];
}) {
  const normalizedSelection = normalizeEvidenceSelection(selection);
  const requestedChunkIds = evidenceLookupChunkIds(
    normalizedSelection,
    context,
  );
  const hasEvidence =
    normalizedSelection.documentIds.length > 0 || requestedChunkIds.length > 0;
  const evidenceQuery = useQuery({
    queryKey: [
      "eval-lab-evidence-state",
      normalizedSelection.documentIds,
      requestedChunkIds,
    ],
    queryFn: ({ signal }) =>
      queryEvalLabEvidence(
        {
          document_ids: normalizedSelection.documentIds,
          chunk_ids: requestedChunkIds,
          include_chunks: false,
          limit:
            normalizedSelection.documentIds.length +
            requestedChunkIds.length +
            10,
        },
        signal,
      ),
    enabled: hasEvidence,
  });

  const isLoadingMetadata = hasEvidence && evidenceQuery.isPending;
  const states = isLoadingMetadata
    ? []
    : deriveEvidenceStates({
        selection: normalizedSelection,
        context,
        metadata: evidenceQuery.isError
          ? { status: "unavailable" }
          : {
              status: "resolved",
              documents: evidenceQuery.data?.documents ?? [],
              chunks: evidenceQuery.data?.chunks ?? [],
              unresolvedDocumentIds:
                evidenceQuery.data?.unresolved_document_ids ?? [],
              unresolvedChunkIds:
                evidenceQuery.data?.unresolved_chunk_ids ?? [],
            },
        failureLabels,
      });

  return (
    <section className={styles.stateList} aria-label={title}>
      <h3>{title}</h3>
      {isLoadingMetadata ? (
        <p className={styles.empty}>Loading evidence metadata…</p>
      ) : states.length > 0 ? (
        states.map((state) => (
          <article
            className={`${styles.stateItem} ${styles[state.severity]}`}
            key={`${state.kind}-${state.evidenceId}`}
          >
            <div className={styles.stateHeader}>
              <strong>{state.label}</strong>
              <span className={styles.badge}>
                {state.kind.replaceAll("_", " ")}
              </span>
            </div>
            <span>{state.description}</span>
            {state.snippet ? <small>{state.snippet}</small> : null}
          </article>
        ))
      ) : (
        <p className={styles.empty}>
          Evidence states appear after expected and retrieved evidence are
          available.
        </p>
      )}
      {evidenceQuery.isError ? (
        <p className={styles.error} role="alert">
          Evidence state lookup failed.
        </p>
      ) : null}
    </section>
  );
}
