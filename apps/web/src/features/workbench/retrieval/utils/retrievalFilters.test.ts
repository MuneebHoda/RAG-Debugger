import { describe, expect, it } from "vitest";

import type {
  DocumentSummary,
  SourceSummary,
} from "../../../../lib/api/sources";
import {
  collectDocuments,
  filterDocumentsBySources,
  retainVisibleDocumentIds,
  toggleSelection,
} from "./retrievalFilters";

const firstDocument = documentSummary("document-1", "source-1");
const secondDocument = documentSummary("document-2", "source-2");

describe("retrieval filters", () => {
  it("collects documents from every source in source order", () => {
    const sources = [
      sourceSummary("source-1", [firstDocument]),
      sourceSummary("source-2", [secondDocument]),
    ];

    expect(collectDocuments(sources)).toEqual([firstDocument, secondDocument]);
  });

  it("filters documents to selected sources", () => {
    expect(
      filterDocumentsBySources([firstDocument, secondDocument], ["source-2"]),
    ).toEqual([secondDocument]);
    expect(
      filterDocumentsBySources([firstDocument, secondDocument], []),
    ).toEqual([firstDocument, secondDocument]);
  });

  it("removes selected document ids hidden by source filters", () => {
    expect(
      retainVisibleDocumentIds(["document-1", "document-2"], [secondDocument]),
    ).toEqual(["document-2"]);
  });

  it("adds and removes selections without mutating the input", () => {
    const original = ["source-1"];

    expect(toggleSelection(original, "source-2")).toEqual([
      "source-1",
      "source-2",
    ]);
    expect(toggleSelection(original, "source-1")).toEqual([]);
    expect(original).toEqual(["source-1"]);
  });
});

function documentSummary(id: string, sourceId: string): DocumentSummary {
  return {
    document: {
      id,
      source_id: sourceId,
      path: `${id}.md`,
      mime_type: "text/markdown",
      checksum: id,
      byte_size: 100,
      profile: "technical_docs",
      extraction_quality: "high",
      warnings: [],
    },
    chunk_count: 2,
  };
}

function sourceSummary(
  id: string,
  documents: DocumentSummary[],
): SourceSummary {
  return {
    source: {
      id,
      project_id: "project-1",
      name: id,
      kind: { FileSet: { root_hint: "test" } },
      sync_policy: "Manual",
      chunking: {
        target_tokens: 512,
        overlap_tokens: 64,
        strategy: "structured",
      },
    },
    document_count: documents.length,
    chunk_count: documents.reduce(
      (total, document) => total + document.chunk_count,
      0,
    ),
    documents,
  };
}
