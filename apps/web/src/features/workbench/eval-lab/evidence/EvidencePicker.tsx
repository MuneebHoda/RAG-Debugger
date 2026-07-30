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
  evidenceSearchError,
  normalizeEvidenceSelection,
  removeEvidenceChunk,
  removeEvidenceDocument,
  type EvidenceHitRef,
  type EvidenceSelection,
} from "./evidenceSelection";
import styles from "./EvidencePicker.module.css";

export interface EvidencePickerProps {
  label?: string;
  sourceIdentity: string;
  query: string;
  selection: EvidenceSelection;
  candidateHits?: EvidenceHitRef[];
  disabled?: boolean;
  onSelectionChange: (selection: EvidenceSelection) => void;
}

export function EvidencePicker({
  label = "Search corpus evidence",
  sourceIdentity,
  query,
  selection,
  candidateHits = [],
  disabled = false,
  onSelectionChange,
}: EvidencePickerProps) {
  const searchId = useId();
  const incomingSearchState = createSearchState(sourceIdentity, query);
  const [searchState, setSearchState] = useState(incomingSearchState);
  const activeSearchState =
    searchState.sourceIdentity === sourceIdentity
      ? searchState
      : incomingSearchState;
  const normalizedSelection = normalizeEvidenceSelection(selection);

  if (searchState.sourceIdentity !== sourceIdentity) {
    setSearchState(incomingSearchState);
  }

  const evidenceQuery = useQuery({
    queryKey: [
      "eval-lab-evidence",
      activeSearchState.submittedQuery,
      normalizedSelection.documentIds,
      normalizedSelection.chunkIds,
    ],
    queryFn: ({ signal }) =>
      queryEvalLabEvidence(
        {
          query: activeSearchState.submittedQuery,
          document_ids: normalizedSelection.documentIds,
          chunk_ids: normalizedSelection.chunkIds,
          include_chunks: true,
          document_limit: 24,
          chunk_limit: 24,
        },
        signal,
      ),
    enabled: !disabled,
  });

  const submitSearch = () => {
    if (disabled) return;

    const nextQuery = activeSearchState.searchInput.trim();
    const validationError = evidenceSearchError(nextQuery);
    if (validationError) {
      setSearchState((current) => ({
        ...current,
        searchError: validationError,
      }));
      return;
    }
    setSearchState((current) => ({
      ...current,
      searchError: null,
      submittedQuery: nextQuery,
    }));
    if (nextQuery === activeSearchState.submittedQuery) {
      void evidenceQuery.refetch();
    }
  };

  return (
    <div className={styles.picker}>
      <form
        className={styles.field}
        onSubmit={(event) => {
          event.preventDefault();
          submitSearch();
        }}
      >
        <label htmlFor={searchId}>{label}</label>
        <span className={styles.searchRow}>
          <input
            id={searchId}
            value={activeSearchState.searchInput}
            aria-describedby={
              activeSearchState.searchError ? `${searchId}-error` : undefined
            }
            aria-invalid={activeSearchState.searchError ? "true" : undefined}
            disabled={disabled}
            onChange={(event) => {
              const searchInput = event.currentTarget.value;
              setSearchState((current) => ({
                ...current,
                searchInput,
                searchError: null,
              }));
            }}
            placeholder="Search path, section, chunk text, or paste an ID"
          />
          <button
            className="secondary-button compact"
            aria-busy={evidenceQuery.isFetching}
            disabled={disabled}
            type="submit"
          >
            {evidenceQuery.isFetching ? (
              <Loader2 aria-hidden="true" className="spin" size={15} />
            ) : (
              <Search aria-hidden="true" size={15} />
            )}
            Search
          </button>
        </span>
      </form>
      {activeSearchState.searchError ? (
        <p className={styles.error} id={`${searchId}-error`} role="alert">
          {activeSearchState.searchError}
        </p>
      ) : null}
      <p className={styles.empty}>
        Use exact chunks when a specific passage must be retrieved. Use
        document-level evidence only when any suitable chunk from that document
        should count.
      </p>

      {candidateHits.length > 0 ? (
        <div
          aria-label="Retrieved evidence from this run"
          className={styles.resultColumn}
          role="region"
        >
          <h3>Retrieved evidence from this run</h3>
          {candidateHits.slice(0, 8).map((hit) => {
            const chunkSelected = normalizedSelection.chunkIds.includes(
              hit.chunkId,
            );
            const documentSelected = normalizedSelection.documentIds.includes(
              hit.documentId,
            );
            return (
              <article
                className={optionClassName({
                  selected: chunkSelected || documentSelected,
                  weak: Boolean(hit.weak),
                  duplicate: Boolean(hit.duplicate),
                })}
                data-selected={chunkSelected || documentSelected}
                key={hit.chunkId}
              >
                <strong>
                  {hit.rank ? `Rank ${hit.rank} · ` : ""}
                  {hit.path ?? hit.label}
                </strong>
                <span>
                  {hit.sectionTitle ? `${hit.sectionTitle} · ` : ""}
                  {hit.snippet ?? compactId(hit.chunkId)}
                </span>
                {hit.weak || hit.duplicate ? (
                  <span className={styles.badgeRow}>
                    {hit.weak ? (
                      <span className={`${styles.badge} ${styles.weak}`}>
                        Weak evidence
                      </span>
                    ) : null}
                    {hit.duplicate ? (
                      <span className={`${styles.badge} ${styles.duplicate}`}>
                        Duplicate evidence
                      </span>
                    ) : null}
                  </span>
                ) : null}
                <span className={styles.actions}>
                  <EvidenceChoiceButton
                    disabled={disabled}
                    label="Expect this exact chunk"
                    selected={chunkSelected}
                    onToggle={() =>
                      onSelectionChange(
                        chunkSelected
                          ? removeEvidenceChunk(selection, hit.chunkId)
                          : addEvidenceChunk(selection, hit.chunkId),
                      )
                    }
                  />
                  <EvidenceChoiceButton
                    disabled={disabled}
                    label="Accept evidence from this document"
                    selected={documentSelected}
                    onToggle={() =>
                      onSelectionChange(
                        documentSelected
                          ? removeEvidenceDocument(selection, hit.documentId)
                          : addEvidenceDocument(selection, hit.documentId),
                      )
                    }
                  />
                </span>
              </article>
            );
          })}
        </div>
      ) : null}

      {evidenceQuery.isFetching ? (
        <p aria-live="polite" className={styles.loading} role="status">
          Searching evidence…
        </p>
      ) : null}
      {evidenceQuery.isError ? (
        <p className={styles.error} role="alert">
          Evidence search failed. Try a narrower query or reload the page.
        </p>
      ) : null}
      {evidenceQuery.isSuccess ? (
        <p
          aria-atomic="true"
          aria-live="polite"
          className={styles.resultCount}
          role="status"
        >
          {evidenceQuery.data.documents.length} document results and{" "}
          {evidenceQuery.data.chunks.length} chunk results.
        </p>
      ) : null}

      <div aria-label="Evidence search results" className={styles.resultGrid}>
        <EvidenceDocumentOptions
          documents={evidenceQuery.data?.documents ?? []}
          disabled={disabled}
          selection={normalizedSelection}
          onToggle={(document, selected) =>
            onSelectionChange(
              selected
                ? removeEvidenceDocument(selection, document.id)
                : addEvidenceDocument(selection, document.id),
            )
          }
        />
        <EvidenceChunkOptions
          chunks={evidenceQuery.data?.chunks ?? []}
          disabled={disabled}
          selection={normalizedSelection}
          onToggle={(chunk, selected) =>
            onSelectionChange(
              selected
                ? removeEvidenceChunk(selection, chunk.id)
                : addEvidenceChunk(selection, chunk.id),
            )
          }
        />
      </div>
    </div>
  );
}

function EvidenceDocumentOptions({
  documents,
  disabled,
  selection,
  onToggle,
}: {
  documents: EvalLabEvidenceDocument[];
  disabled: boolean;
  selection: EvidenceSelection;
  onToggle: (document: EvalLabEvidenceDocument, selected: boolean) => void;
}) {
  return (
    <div className={styles.resultColumn}>
      <h3>Documents</h3>
      {documents.length > 0 ? (
        documents.map((document) => {
          const selected = selection.documentIds.includes(document.id);
          return (
            <article
              className={optionClassName({ selected })}
              data-selected={selected}
              key={document.id}
            >
              <strong>{document.path}</strong>
              <span>
                {document.source_name} · {document.profile.replaceAll("_", " ")}{" "}
                · {document.chunk_count} chunks
              </span>
              <EvidenceChoiceButton
                disabled={disabled}
                label="Accept evidence from this document"
                selected={selected}
                onToggle={() => onToggle(document, selected)}
              />
            </article>
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
  disabled,
  selection,
  onToggle,
}: {
  chunks: EvalLabEvidenceChunk[];
  disabled: boolean;
  selection: EvidenceSelection;
  onToggle: (chunk: EvalLabEvidenceChunk, selected: boolean) => void;
}) {
  return (
    <div className={styles.resultColumn}>
      <h3>Chunks</h3>
      {chunks.length > 0 ? (
        chunks.map((chunk) => {
          const selected = selection.chunkIds.includes(chunk.id);
          return (
            <article
              className={optionClassName({
                selected,
                duplicate: chunk.is_duplicate,
                weak: chunk.quality_flags.includes("too_short"),
              })}
              data-selected={selected}
              key={chunk.id}
            >
              <strong>
                {chunk.document_path} · chunk {chunk.ordinal + 1}
              </strong>
              <span>
                {chunk.section_title ? `${chunk.section_title} · ` : ""}
                {chunk.text_preview}
                {chunk.preview_truncated ? "…" : ""}
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
              <EvidenceChoiceButton
                disabled={disabled}
                label="Expect this exact chunk"
                selected={selected}
                onToggle={() => onToggle(chunk, selected)}
              />
            </article>
          );
        })
      ) : (
        <p className={styles.empty}>No matching chunks yet.</p>
      )}
    </div>
  );
}

function EvidenceChoiceButton({
  disabled,
  label,
  selected,
  onToggle,
}: {
  disabled: boolean;
  label: string;
  selected: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      aria-label={label}
      aria-pressed={selected}
      className={`secondary-button compact ${styles.choice} ${
        selected ? styles.selected : styles.unselected
      }`}
      disabled={disabled}
      type="button"
      onClick={onToggle}
    >
      <span>{label}</span>
      <span className={styles.choiceState}>
        {selected ? "Selected" : "Not selected"}
      </span>
    </button>
  );
}

function initialSearchQuery(query: string): string {
  const trimmed = query.trim();
  return evidenceSearchError(trimmed) ? "" : trimmed;
}

function createSearchState(sourceIdentity: string, query: string) {
  return {
    sourceIdentity,
    searchInput: query,
    submittedQuery: initialSearchQuery(query),
    searchError: null as string | null,
  };
}

function optionClassName({
  selected,
  weak = false,
  duplicate = false,
}: {
  selected: boolean;
  weak?: boolean;
  duplicate?: boolean;
}): string {
  return [
    styles.option,
    selected ? styles.selected : styles.unselected,
    weak ? styles.weak : "",
    duplicate ? styles.duplicate : "",
  ]
    .filter(Boolean)
    .join(" ");
}
