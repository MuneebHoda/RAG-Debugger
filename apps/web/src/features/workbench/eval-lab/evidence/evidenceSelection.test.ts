import { describe, expect, it } from "vitest";

import type {
  EvalLabEvidenceChunk,
  EvalLabEvidenceDocument,
  RetrievalEvalCase,
} from "../../../../lib/api/evalLab";
import {
  addEvidenceChunk,
  addEvidenceDocument,
  buildUpdateCaseEvidencePayload,
  deriveEvidenceStates,
  evidenceSelectionsEqual,
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

  it("does not infer document expectations from exact chunk expectations", () => {
    expect(
      addEvidenceChunk({ documentIds: [], chunkIds: [] }, "chunk-1"),
    ).toEqual({
      documentIds: [],
      chunkIds: ["chunk-1"],
    });
    expect(
      addEvidenceDocument({ documentIds: [], chunkIds: [] }, "doc-1"),
    ).toEqual({
      documentIds: ["doc-1"],
      chunkIds: [],
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

  it("compares normalized evidence sets without depending on order", () => {
    expect(
      evidenceSelectionsEqual(
        {
          documentIds: ["doc-2", "doc-1", "doc-1"],
          chunkIds: ["chunk-2", "chunk-1"],
        },
        {
          documentIds: ["doc-1", "doc-2"],
          chunkIds: ["chunk-1", "chunk-2", "chunk-2"],
        },
      ),
    ).toBe(true);
    expect(
      evidenceSelectionsEqual(
        { documentIds: ["doc-1"], chunkIds: ["chunk-1"] },
        { documentIds: ["doc-1"], chunkIds: ["chunk-2"] },
      ),
    ).toBe(false);
  });

  it("omits unchanged evidence and sends complete normalized replacements", () => {
    const original = {
      documentIds: ["doc-1"],
      chunkIds: ["chunk-1"],
    };

    expect(
      buildUpdateCaseEvidencePayload(original, {
        documentIds: ["doc-1", "doc-1"],
        chunkIds: ["chunk-1"],
      }),
    ).toEqual({});
    expect(
      buildUpdateCaseEvidencePayload(original, {
        documentIds: ["doc-2", "doc-2"],
        chunkIds: ["chunk-2", "chunk-2"],
      }),
    ).toEqual({
      expected_document_ids: ["doc-2"],
      expected_chunk_ids: ["chunk-2"],
    });
    expect(
      buildUpdateCaseEvidencePayload(original, {
        documentIds: [],
        chunkIds: [],
      }),
    ).toEqual({
      expected_document_ids: [],
      expected_chunk_ids: [],
    });
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

  it("shows neutral expectations when no retrieval context exists", () => {
    const states = deriveEvidenceStates({
      selection: normalizeEvidenceSelection({
        documentIds: ["doc-1"],
        chunkIds: ["chunk-1"],
      }),
      context: { kind: "expectation_only" },
      metadata: resolvedMetadata({
        chunks: [chunkFixture("chunk-1", "doc-1")],
      }),
    });

    expect(states.map((state) => state.kind)).toEqual([
      "expected_exact_chunk",
      "expected_document",
    ]);
    expect(states.map((state) => state.label)).toEqual([
      "Expected exact chunk",
      "Expected document",
    ]);
  });

  it("marks expectations missing after a completed retrieval with zero hits", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: ["doc-1"], chunkIds: ["chunk-1"] },
      context: { kind: "retrieval", hits: [] },
      metadata: resolvedMetadata({
        chunks: [chunkFixture("chunk-1", "doc-1")],
      }),
    });

    expect(states.map((state) => state.kind)).toEqual([
      "expected_missing",
      "expected_missing",
    ]);
  });

  it("satisfies a document expectation through a retrieved child chunk", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: ["doc-1"], chunkIds: [] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-2", rank: 1 }],
      },
      metadata: resolvedMetadata({
        documents: [],
        chunks: [chunkFixture("chunk-2", "doc-1")],
      }),
    });

    expect(states).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: "expected_retrieved",
          evidenceId: "doc-1",
        }),
      ]),
    );
  });

  it("emits one metadata-derived wrong-chunk state when the failure label agrees", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: [], chunkIds: ["chunk-1"] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-2", rank: 1 }],
      },
      metadata: resolvedMetadata({
        chunks: [
          chunkFixture("chunk-1", "doc-1"),
          chunkFixture("chunk-2", "doc-1"),
        ],
      }),
      failureLabels: ["correct_document_wrong_chunk"],
    });

    expect(states.filter((state) => state.kind === "wrong_chunk")).toEqual([
      expect.objectContaining({
        kind: "wrong_chunk",
        evidenceId: "chunk-1",
        label: "Expected document retrieved, wrong chunk",
      }),
    ]);
    expect(states).toContainEqual(
      expect.objectContaining({
        kind: "retrieved_not_expected",
        evidenceId: "chunk-2",
      }),
    );
  });

  it("does not infer wrong-chunk from a failure label when expected metadata is unresolved", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: [], chunkIds: ["chunk-stale"] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-2", rank: 1 }],
      },
      metadata: resolvedMetadata({
        documents: [documentFixtureFor("doc-2", "other.md")],
        chunks: [chunkFixture("chunk-2", "doc-2")],
        unresolvedChunkIds: ["chunk-stale"],
      }),
      failureLabels: ["correct_document_wrong_chunk"],
    });

    expect(states).toContainEqual(
      expect.objectContaining({
        kind: "stale_evidence",
        evidenceId: "chunk-stale",
      }),
    );
    expect(states.some((state) => state.kind === "wrong_chunk")).toBe(false);
  });

  it("keeps retrieved metadata authoritative over an inconsistent wrong-chunk label", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: [], chunkIds: ["chunk-1"] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-1", rank: 1 }],
      },
      metadata: resolvedMetadata({
        chunks: [chunkFixture("chunk-1", "doc-1")],
      }),
      failureLabels: ["correct_document_wrong_chunk"],
    });

    expect(states).toContainEqual(
      expect.objectContaining({
        kind: "expected_retrieved",
        evidenceId: "chunk-1",
      }),
    );
    expect(states.some((state) => state.kind === "wrong_chunk")).toBe(false);
  });

  it("keeps unrelated retrieved chunks separate from missing expectations", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: ["doc-1"], chunkIds: ["chunk-1"] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-3", rank: 1 }],
      },
      metadata: resolvedMetadata({
        documents: [documentFixture, documentFixtureFor("doc-2", "other.md")],
        chunks: [
          chunkFixture("chunk-1", "doc-1"),
          chunkFixture("chunk-3", "doc-2"),
        ],
      }),
    });

    expect(
      states.filter((state) => state.kind === "expected_missing"),
    ).toHaveLength(2);
    expect(states).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: "retrieved_not_expected",
          evidenceId: "chunk-3",
        }),
      ]),
    );
  });

  it("keeps mixed document and exact-chunk expectations independent", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: ["doc-1"], chunkIds: ["chunk-1"] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-2", duplicate: true, weak: true }],
      },
      metadata: resolvedMetadata({
        chunks: [
          chunkFixture("chunk-1", "doc-1"),
          chunkFixture("chunk-2", "doc-1"),
        ],
      }),
    });

    expect(states).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ kind: "wrong_chunk", evidenceId: "chunk-1" }),
        expect.objectContaining({
          kind: "expected_retrieved",
          evidenceId: "doc-1",
        }),
        expect.objectContaining({
          kind: "duplicate_evidence",
          evidenceId: "chunk-2",
        }),
        expect.objectContaining({
          kind: "weak_evidence",
          evidenceId: "chunk-2",
        }),
      ]),
    );
  });

  it("represents unavailable metadata without inventing retrieval outcomes", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: ["doc-1"], chunkIds: ["chunk-1"] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-2", rank: 1 }],
      },
      metadata: { status: "unavailable" },
    });

    expect(states.every((state) => state.kind === "metadata_unavailable")).toBe(
      true,
    );
  });

  it("distinguishes stale expectations from unresolved retrieved metadata", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: [], chunkIds: ["chunk-stale"] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-unavailable" }],
      },
      metadata: resolvedMetadata({
        unresolvedChunkIds: ["chunk-stale", "chunk-unavailable"],
      }),
    });

    expect(states).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: "stale_evidence",
          evidenceId: "chunk-stale",
        }),
        expect.objectContaining({
          kind: "metadata_unavailable",
          evidenceId: "chunk-unavailable",
        }),
      ]),
    );
  });

  it("never marks the same expectation both retrieved and missing", () => {
    const states = deriveEvidenceStates({
      selection: { documentIds: ["doc-1"], chunkIds: ["chunk-1"] },
      context: {
        kind: "retrieval",
        hits: [{ chunkId: "chunk-1" }],
      },
      metadata: resolvedMetadata({
        chunks: [chunkFixture("chunk-1", "doc-1")],
      }),
    });

    for (const evidenceId of ["doc-1", "chunk-1"]) {
      const primaryKinds = states
        .filter((state) => state.evidenceId === evidenceId)
        .map((state) => state.kind)
        .filter((kind) =>
          ["expected_retrieved", "expected_missing", "wrong_chunk"].includes(
            kind,
          ),
        );
      expect(primaryKinds).toEqual(["expected_retrieved"]);
    }
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

function documentFixtureFor(id: string, path: string): EvalLabEvidenceDocument {
  return { ...documentFixture, id, path };
}

function resolvedMetadata({
  documents = [documentFixture],
  chunks = [],
  unresolvedDocumentIds = [],
  unresolvedChunkIds = [],
}: {
  documents?: EvalLabEvidenceDocument[];
  chunks?: EvalLabEvidenceChunk[];
  unresolvedDocumentIds?: string[];
  unresolvedChunkIds?: string[];
} = {}) {
  return {
    status: "resolved" as const,
    documents,
    chunks,
    unresolvedDocumentIds,
    unresolvedChunkIds,
  };
}

function chunkFixture(id: string, documentId: string): EvalLabEvidenceChunk {
  return {
    id,
    document_id: documentId,
    source_id: "source-1",
    source_name: "Source",
    document_path: "platform-guide.md",
    ordinal: id === "chunk-1" ? 0 : 1,
    text_preview: "GPU indexing refreshes local embeddings.",
    preview_truncated: false,
    token_count: 8,
    checksum: "checksum",
    section_title: "GPU Indexing",
    quality_flags: [],
    is_duplicate: false,
    text_density: 0.9,
    evidence_score_hint: 0.8,
  };
}
