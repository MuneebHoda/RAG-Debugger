import { X } from "lucide-react";
import { useQuery } from "@tanstack/react-query";

import { queryEvalLabEvidence } from "../../../../lib/api/evalLab";
import {
  compactId,
  normalizeEvidenceSelection,
  removeEvidenceChunk,
  removeEvidenceDocument,
  type EvidenceSelection,
} from "./evidenceSelection";
import styles from "./EvidencePicker.module.css";

export function EvidenceSelectionReview({
  selection,
  onSelectionChange,
  allowEmptySelection = false,
  disabled = false,
}: {
  selection: EvidenceSelection;
  onSelectionChange: (selection: EvidenceSelection) => void;
  allowEmptySelection?: boolean;
  disabled?: boolean;
}) {
  const normalizedSelection = normalizeEvidenceSelection(selection);
  const evidenceQuery = useQuery({
    queryKey: [
      "eval-lab-evidence-review",
      normalizedSelection.documentIds,
      normalizedSelection.chunkIds,
    ],
    queryFn: ({ signal }) =>
      queryEvalLabEvidence(
        {
          document_ids: normalizedSelection.documentIds,
          chunk_ids: normalizedSelection.chunkIds,
          include_chunks: true,
          document_limit: 0,
          chunk_limit: 0,
        },
        signal,
      ),
    enabled:
      normalizedSelection.documentIds.length > 0 ||
      normalizedSelection.chunkIds.length > 0,
  });
  const resolvedDocumentIds = new Set(
    (evidenceQuery.data?.documents ?? []).map((document) => document.id),
  );
  const resolvedChunkIds = new Set(
    (evidenceQuery.data?.chunks ?? []).map((chunk) => chunk.id),
  );
  const unavailableDocumentIds = evidenceQuery.isError
    ? normalizedSelection.documentIds.filter(
        (id) => !resolvedDocumentIds.has(id),
      )
    : [];
  const unavailableChunkIds = evidenceQuery.isError
    ? normalizedSelection.chunkIds.filter((id) => !resolvedChunkIds.has(id))
    : [];

  if (
    normalizedSelection.documentIds.length === 0 &&
    normalizedSelection.chunkIds.length === 0
  ) {
    return (
      <div aria-live="polite" className={styles.warning} role="status">
        {allowEmptySelection
          ? "No expected evidence is selected. Saving will clear this case's evidence, so it will not measure retrieval quality until evidence is added."
          : "Select at least one expected document or chunk before saving. Good eval cases need evidence they can measure."}
      </div>
    );
  }

  return (
    <div className={styles.review}>
      <h3>Selected expected evidence</h3>
      <p className={styles.empty}>
        Exact chunks require that specific chunk to be retrieved. Document-level
        evidence accepts any suitable chunk from that document.
      </p>
      {evidenceQuery.isPending ? (
        <p aria-live="polite" className={styles.loading} role="status">
          Loading selected evidence metadata…
        </p>
      ) : null}
      <div className={styles.selectedList}>
        {(evidenceQuery.data?.documents ?? []).map((document) => (
          <article
            className={`${styles.selectedItem} ${styles.selected}`}
            key={document.id}
          >
            <div className={styles.selectedHeader}>
              <strong>{document.path}</strong>
              <button
                aria-label={`Remove document ${document.path}`}
                className="secondary-button compact"
                disabled={disabled}
                type="button"
                onClick={() =>
                  onSelectionChange(
                    removeEvidenceDocument(selection, document.id),
                  )
                }
              >
                <X aria-hidden="true" size={14} />
                Remove
              </button>
            </div>
            <span>
              Document-level expectation · accepts evidence from{" "}
              {document.source_name} · {document.chunk_count} chunks
            </span>
          </article>
        ))}
        {(evidenceQuery.data?.chunks ?? []).map((chunk) => (
          <article
            className={`${styles.selectedItem} ${styles.selected}`}
            key={chunk.id}
          >
            <div className={styles.selectedHeader}>
              <strong>
                {chunk.document_path} · chunk {chunk.ordinal + 1}
              </strong>
              <button
                aria-label={`Remove chunk ${chunk.ordinal + 1}`}
                className="secondary-button compact"
                disabled={disabled}
                type="button"
                onClick={() =>
                  onSelectionChange(removeEvidenceChunk(selection, chunk.id))
                }
              >
                <X aria-hidden="true" size={14} />
                Remove
              </button>
            </div>
            <span>
              Exact chunk expectation · this chunk must be retrieved ·{" "}
              {chunk.section_title ? `${chunk.section_title} · ` : ""}
              checksum {chunk.checksum.slice(0, 12)}
            </span>
          </article>
        ))}
        {(evidenceQuery.data?.unresolved_document_ids ?? []).map((id) => (
          <article
            className={`${styles.selectedItem} ${styles.stale}`}
            key={id}
          >
            <div className={styles.selectedHeader}>
              <strong>Stale/deleted expected document</strong>
              <button
                aria-label={`Remove stale document ${compactId(id)}`}
                className="secondary-button compact"
                disabled={disabled}
                type="button"
                onClick={() =>
                  onSelectionChange(removeEvidenceDocument(selection, id))
                }
              >
                <X aria-hidden="true" size={14} />
                Remove stale document
              </button>
            </div>
            <span>{compactId(id)} is no longer resolvable.</span>
          </article>
        ))}
        {(evidenceQuery.data?.unresolved_chunk_ids ?? []).map((id) => (
          <article
            className={`${styles.selectedItem} ${styles.stale}`}
            key={id}
          >
            <div className={styles.selectedHeader}>
              <strong>Stale/deleted expected chunk</strong>
              <button
                aria-label={`Remove stale chunk ${compactId(id)}`}
                className="secondary-button compact"
                disabled={disabled}
                type="button"
                onClick={() =>
                  onSelectionChange(removeEvidenceChunk(selection, id))
                }
              >
                <X aria-hidden="true" size={14} />
                Remove stale chunk
              </button>
            </div>
            <span>{compactId(id)} is no longer resolvable.</span>
          </article>
        ))}
        {unavailableDocumentIds.map((id) => (
          <article
            className={`${styles.selectedItem} ${styles.warning}`}
            key={`unavailable-document-${id}`}
          >
            <div className={styles.selectedHeader}>
              <strong>Document metadata unavailable</strong>
              <button
                aria-label={`Remove document ${compactId(id)}`}
                className="secondary-button compact"
                disabled={disabled}
                type="button"
                onClick={() =>
                  onSelectionChange(removeEvidenceDocument(selection, id))
                }
              >
                <X aria-hidden="true" size={14} />
                Remove document
              </button>
            </div>
            <span>
              {compactId(id)} remains selected. Retry the lookup or remove it.
            </span>
          </article>
        ))}
        {unavailableChunkIds.map((id) => (
          <article
            className={`${styles.selectedItem} ${styles.warning}`}
            key={`unavailable-chunk-${id}`}
          >
            <div className={styles.selectedHeader}>
              <strong>Chunk metadata unavailable</strong>
              <button
                aria-label={`Remove chunk ${compactId(id)}`}
                className="secondary-button compact"
                disabled={disabled}
                type="button"
                onClick={() =>
                  onSelectionChange(removeEvidenceChunk(selection, id))
                }
              >
                <X aria-hidden="true" size={14} />
                Remove chunk
              </button>
            </div>
            <span>
              {compactId(id)} remains selected. Retry the lookup or remove it.
            </span>
          </article>
        ))}
      </div>
      {evidenceQuery.isError ? (
        <p className={styles.error} role="alert">
          Selected evidence could not be resolved.
        </p>
      ) : null}
    </div>
  );
}
