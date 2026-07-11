import { describe, expect, it } from "vitest";

import type {
  EvalLabEvidenceChunk,
  EvalLabEvidenceDocument,
  RetrievalEvalCase,
} from "../../../../lib/api/evalLab";
import {
  addEvidenceChunk,
  addEvidenceDocument,
  deriveEvidenceStates,
  findSimilarCases,
  normalizeEvidenceSelection,
  selectionFromCase,
} from "./evidenceSelection";

describe("evidence selection helpers", () => {
  it("deduplicates documents and chunks when adding evidence", () => {
    const selection = addEvidenceChunk(
      addEvidenceChunk(
        addEvidenceDocument(
          addEvidenceDocument({ documentIds: [], chunkIds: [] }, "doc-1"),
          "doc-1",
        ),
        "chunk-1",
      ),
      "chunk-1",
    );

    expect(selection).toEqual({
      documentIds: ["doc-1"],
      chunkIds: ["chunk-1"],
    });
  });

  it("normalizes an existing eval case selection", () => {
    expect(
      selectionFromCase({
        ...caseFixture,
        expected_document_ids: ["doc-1", "doc-1"],
        expected_chunk_ids: ["chunk-1", "chunk-1"],
      }),
    ).toEqual({ documentIds: ["doc-1"], chunkIds: ["chunk-1"] });
  });

  it("detects similar cases by normalized query", () => {
    const matches = findSimilarCases(
      [
        { ...caseFixture, id: "case-1", query: "How does GPU indexing work?" },
        { ...caseFixture, id: "case-2", query: "Different question" },
      ],
      "  how does gpu indexing work? ",
    );

    expect(matches.map((match) => match.id)).toEqual(["case-1"]);
  });

  it("derives expected, missing, wrong-chunk, weak, duplicate, and stale states", () => {
    const states = deriveEvidenceStates({
      selection: normalizeEvidenceSelection({
        documentIds: ["doc-1"],
        chunkIds: ["chunk-1", "chunk-2"],
      }),
      documents: [documentFixture],
      chunks: [
        chunkFixture("chunk-1", "doc-1"),
        chunkFixture("chunk-2", "doc-1"),
      ],
      retrievedHits: [
        {
          label: "[1]",
          chunkId: "chunk-1",
          documentId: "doc-1",
          duplicate: true,
          weak: true,
        },
        {
          label: "[2]",
          chunkId: "chunk-3",
          documentId: "doc-2",
        },
      ],
      unresolvedChunkIds: ["chunk-stale"],
    });

    expect(states.map((state) => state.kind)).toEqual(
      expect.arrayContaining([
        "expected_retrieved",
        "wrong_chunk",
        "duplicate_evidence",
        "weak_evidence",
        "stale_evidence",
        "retrieved_not_expected",
      ]),
    );
  });
});

const caseFixture: RetrievalEvalCase = {
  id: "case-1",
  name: "GPU indexing",
  query: "How does GPU indexing work?",
  top_k: 5,
  expected_chunk_ids: ["chunk-1"],
  expected_document_ids: ["doc-1"],
  notes: null,
  created_at: "2026-06-25T00:00:00Z",
};

const documentFixture: EvalLabEvidenceDocument = {
  id: "doc-1",
  source_id: "source-1",
  source_name: "Source",
  path: "platform-guide.md",
  profile: "technical_docs",
  extraction_quality: "high",
  warnings: [],
  chunk_count: 3,
};

function chunkFixture(id: string, documentId: string): EvalLabEvidenceChunk {
  return {
    id,
    document_id: documentId,
    source_id: "source-1",
    source_name: "Source",
    document_path: "platform-guide.md",
    ordinal: id === "chunk-1" ? 0 : 1,
    text: "GPU indexing refreshes local embeddings.",
    token_count: 8,
    checksum: "checksum",
    section_title: "GPU Indexing",
    quality_flags: [],
    is_duplicate: false,
    text_density: 0.9,
    evidence_score_hint: 0.8,
  };
}
