import type {
  DocumentSummary,
  SourceSummary,
} from "../../../../lib/api/sources";

export function collectDocuments(sources: SourceSummary[]): DocumentSummary[] {
  return sources.flatMap((source) => source.documents);
}

export function filterDocumentsBySources(
  documents: DocumentSummary[],
  selectedSourceIds: string[],
): DocumentSummary[] {
  if (selectedSourceIds.length === 0) return documents;

  const selectedSources = new Set(selectedSourceIds);
  return documents.filter(({ document }) =>
    selectedSources.has(document.source_id),
  );
}

export function retainVisibleDocumentIds(
  selectedDocumentIds: string[],
  visibleDocuments: DocumentSummary[],
): string[] {
  const visibleIds = new Set(
    visibleDocuments.map(({ document }) => document.id),
  );
  return selectedDocumentIds.filter((documentId) => visibleIds.has(documentId));
}

export function toggleSelection(currentIds: string[], id: string): string[] {
  return currentIds.includes(id)
    ? currentIds.filter((currentId) => currentId !== id)
    : [...currentIds, id];
}
