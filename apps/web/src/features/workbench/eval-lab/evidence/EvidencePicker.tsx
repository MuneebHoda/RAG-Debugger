import { Loader2, Search } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useId, useState } from "react";

import {
  queryEvalLabEvidence,
  type EvalLabEvidenceChunk,
  type EvalLabEvidenceDocument,
} from "../../../../lib/api/evalLab";
import {
  addEvidenceChunk,
  addEvidenceDocument,
  compactId,
  normalizeEvidenceSelection,
  type EvidenceHitRef,
  type EvidenceSelection,
} from "./evidenceSelection";
import styles from "./EvidencePicker.module.css";

export interface EvidencePickerProps {
  label?: string;
  query: string;
  selection: EvidenceSelection;
  candidateHits?: EvidenceHitRef[];
  onSelectionChange: (selection: EvidenceSelection) => void;
}

export function EvidencePicker({
  label = "Search corpus evidence",
  query,
  selection,
  candidateHits = [],
  onSelectionChange,
}: EvidencePickerProps) {
  const searchId = useId();
  const [search, setSearch] = useState(query);
  const normalizedSelection = normalizeEvidenceSelection(selection);
  const evidenceQuery = useQuery({
    queryKey: [
      "eval-lab-evidence",
      search,
      normalizedSelection.documentIds,
      normalizedSelection.chunkIds,
    ],
    queryFn: ({ signal }) =>
      queryEvalLabEvidence(
        {
          query: search,
          document_ids: normalizedSelection.documentIds,
          chunk_ids: normalizedSelection.chunkIds,
          include_chunks: true,
          limit: 24,
        },
        signal,
      ),
  });

  return (
    <div className={styles.picker}>
      <label className={styles.field} htmlFor={searchId}>
        {label}
        <span className={styles.searchRow}>
          <input
            id={searchId}
            value={search}
            onChange={(event) => setSearch(event.currentTarget.value)}
            placeholder="Search path, section, chunk text, or paste an ID"
          />
          <button
            className="secondary-button compact"
            disabled={evidenceQuery.isFetching}
            type="button"
            onClick={() => void evidenceQuery.refetch()}
          >
            {evidenceQuery.isFetching ? (
              <Loader2 aria-hidden="true" className="spin" size={15} />
            ) : (
              <Search aria-hidden="true" size={15} />
            )}
            Search
          </button>
        </span>
      </label>

      {candidateHits.length > 0 ? (
        <div className={styles.resultColumn}>
          <h3>Retrieved evidence from this run</h3>
          {candidateHits.slice(0, 8).map((hit) => (
            <button
              className={styles.option}
              key={hit.chunkId}
              type="button"
              onClick={() =>
                onSelectionChange(
                  addEvidenceChunk(
                    addEvidenceDocument(selection, hit.documentId),
                    hit.chunkId,
                  ),
                )
              }
            >
              <strong>
                {hit.rank ? `Rank ${hit.rank} · ` : ""}
                {hit.path ?? hit.label}
              </strong>
              <span>
                {hit.sectionTitle ? `${hit.sectionTitle} · ` : ""}
                {hit.snippet ?? compactId(hit.chunkId)}
              </span>
            </button>
          ))}
        </div>
      ) : null}

      {evidenceQuery.isError ? (
        <p className={styles.error} role="alert">
          Evidence search failed. Try a narrower query or reload the page.
        </p>
      ) : null}

      <div className={styles.resultGrid}>
        <EvidenceDocumentOptions
          documents={evidenceQuery.data?.documents ?? []}
          selection={normalizedSelection}
          onAdd={(document) =>
            onSelectionChange(addEvidenceDocument(selection, document.id))
          }
        />
        <EvidenceChunkOptions
          chunks={evidenceQuery.data?.chunks ?? []}
          selection={normalizedSelection}
          onAdd={(chunk) =>
            onSelectionChange(
              addEvidenceChunk(
                addEvidenceDocument(selection, chunk.document_id),
                chunk.id,
              ),
            )
          }
        />
      </div>
    </div>
  );
}

function EvidenceDocumentOptions({
  documents,
  selection,
  onAdd,
}: {
  documents: EvalLabEvidenceDocument[];
  selection: EvidenceSelection;
  onAdd: (document: EvalLabEvidenceDocument) => void;
}) {
  return (
    <div className={styles.resultColumn}>
      <h3>Documents</h3>
      {documents.length > 0 ? (
        documents.map((document) => {
          const selected = selection.documentIds.includes(document.id);
          return (
            <button
              className={styles.option}
              disabled={selected}
              key={document.id}
              type="button"
              onClick={() => onAdd(document)}
            >
              <strong>{document.path}</strong>
              <span>
                {document.source_name} · {document.profile.replaceAll("_", " ")}{" "}
                · {document.chunk_count} chunks
              </span>
            </button>
          );
        })
      ) : (
        <p className={styles.empty}>No matching documents yet.</p>
      )}
    </div>
  );
}

function EvidenceChunkOptions({
  chunks,
  selection,
  onAdd,
}: {
  chunks: EvalLabEvidenceChunk[];
  selection: EvidenceSelection;
  onAdd: (chunk: EvalLabEvidenceChunk) => void;
}) {
  return (
    <div className={styles.resultColumn}>
      <h3>Chunks</h3>
      {chunks.length > 0 ? (
        chunks.map((chunk) => {
          const selected = selection.chunkIds.includes(chunk.id);
          return (
            <button
              className={styles.option}
              disabled={selected}
              key={chunk.id}
              type="button"
              onClick={() => onAdd(chunk)}
            >
              <strong>
                {chunk.document_path} · chunk {chunk.ordinal + 1}
              </strong>
              <span>
                {chunk.section_title ? `${chunk.section_title} · ` : ""}
                {chunk.text.slice(0, 150)}
              </span>
              {chunk.quality_flags.length > 0 ? (
                <span className={styles.badgeRow}>
                  {chunk.quality_flags.map((flag) => (
                    <span className={styles.badge} key={flag}>
                      {flag.replaceAll("_", " ")}
                    </span>
                  ))}
                </span>
              ) : null}
            </button>
          );
        })
      ) : (
        <p className={styles.empty}>No matching chunks yet.</p>
      )}
    </div>
  );
}
