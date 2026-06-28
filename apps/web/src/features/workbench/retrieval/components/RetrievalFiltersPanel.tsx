import { SlidersHorizontal } from "lucide-react";

import type {
  DocumentSummary,
  SourceSummary,
} from "../../../../lib/api/sources";
import styles from "../RetrievalPage.module.css";

export function RetrievalFiltersPanel({
  documents,
  selectedDocumentIds,
  selectedSourceIds,
  sources,
  onToggleDocument,
  onToggleSource,
}: {
  documents: DocumentSummary[];
  selectedDocumentIds: string[];
  selectedSourceIds: string[];
  sources: SourceSummary[];
  onToggleDocument: (documentId: string) => void;
  onToggleSource: (sourceId: string) => void;
}) {
  return (
    <div className={styles.filterStack}>
      <div className={styles.filterHeading}>
        <SlidersHorizontal aria-hidden="true" size={16} />
        <strong>Filters</strong>
      </div>

      <div className={styles.checkboxList} aria-label="Source filters">
        {sources.length === 0 ? (
          <span>No sources indexed yet.</span>
        ) : (
          sources.map((source) => (
            <label key={source.source.id}>
              <input
                checked={selectedSourceIds.includes(source.source.id)}
                type="checkbox"
                onChange={() => onToggleSource(source.source.id)}
              />
              <span>{source.source.name}</span>
            </label>
          ))
        )}
      </div>

      <div className={styles.checkboxList} aria-label="Document filters">
        {documents.length === 0 ? (
          <span>No documents available for the selected source filter.</span>
        ) : (
          documents.map(({ document, chunk_count }) => (
            <label key={document.id}>
              <input
                checked={selectedDocumentIds.includes(document.id)}
                type="checkbox"
                onChange={() => onToggleDocument(document.id)}
              />
              <span>
                {document.path} · {chunk_count} chunks
              </span>
            </label>
          ))
        )}
      </div>
    </div>
  );
}
