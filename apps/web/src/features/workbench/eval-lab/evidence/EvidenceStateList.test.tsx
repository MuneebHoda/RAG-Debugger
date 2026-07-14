import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { EvidenceStateList } from "./EvidenceStateList";
import type { EvidenceStateContext } from "./evidenceSelection";

const documentId = "018f7a2a-6e2e-7000-a000-000000000401";
const expectedChunkId = "018f7a2a-6e2e-7000-a000-000000000402";
const retrievedChunkId = "018f7a2a-6e2e-7000-a000-000000000403";

describe("EvidenceStateList", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("queries expected and retrieved evidence together and uses resolved metadata", async () => {
    let requestBody: Record<string, unknown> | null = null;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
        requestBody = JSON.parse(String(init?.body)) as Record<string, unknown>;
        return responseJson(evidenceLookup());
      }),
    );

    renderStateList({
      context: {
        kind: "retrieval",
        hits: [{ chunkId: retrievedChunkId, rank: 1 }],
      },
    });

    expect(
      await screen.findByText("Expected document retrieved"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Expected document retrieved, wrong chunk"),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/platform-guide\.md/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/GPU Indexing/).length).toBeGreaterThan(0);
    expect(requestBody).toMatchObject({
      document_ids: [documentId],
      chunk_ids: [expectedChunkId, retrievedChunkId],
      include_chunks: false,
    });
  });

  it("renders neutral saved expectations without retrieval claims", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => responseJson(evidenceLookup())),
    );

    renderStateList({ context: { kind: "expectation_only" } });

    expect(await screen.findByText("Expected exact chunk")).toBeInTheDocument();
    expect(screen.getByText("Expected document")).toBeInTheDocument();
    expect(screen.queryByText(/missing/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/wrong chunk/i)).not.toBeInTheDocument();
  });

  it("keeps classifications unknown when metadata lookup fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => responseError(500)),
    );

    renderStateList({
      context: {
        kind: "retrieval",
        hits: [{ chunkId: retrievedChunkId, rank: 1 }],
      },
    });

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Evidence state lookup failed",
    );
    expect(
      screen.getByText("Expected exact chunk metadata unavailable"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Expected document metadata unavailable"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Retrieved evidence metadata unavailable"),
    ).toBeInTheDocument();
    expect(screen.queryByText("Expected but missing")).not.toBeInTheDocument();
    expect(screen.queryByText(/wrong chunk/i)).not.toBeInTheDocument();
  });
});

function renderStateList({ context }: { context: EvidenceStateContext }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <EvidenceStateList
        context={context}
        selection={{
          documentIds: [documentId],
          chunkIds: [expectedChunkId],
        }}
      />
    </QueryClientProvider>,
  );
}

function evidenceLookup() {
  return {
    documents: [
      {
        id: documentId,
        source_id: "source-1",
        source_name: "Corpus",
        path: "platform-guide.md",
        profile: "technical_docs",
        extraction_quality: "high",
        warnings: [],
        chunk_count: 2,
      },
    ],
    chunks: [
      evidenceChunk(expectedChunkId, 0, "Expected indexing behavior."),
      evidenceChunk(retrievedChunkId, 1, "Retrieved sibling evidence."),
    ],
    unresolved_document_ids: [],
    unresolved_chunk_ids: [],
  };
}

function evidenceChunk(id: string, ordinal: number, text: string) {
  return {
    id,
    document_id: documentId,
    source_id: "source-1",
    source_name: "Corpus",
    document_path: "platform-guide.md",
    ordinal,
    text,
    token_count: 4,
    checksum: `checksum-${ordinal}`,
    section_title: "GPU Indexing",
    quality_flags: [],
    is_duplicate: false,
    text_density: 0.9,
    evidence_score_hint: 0.8,
  };
}

function responseJson(json: unknown) {
  return Promise.resolve({ status: 200, json: async () => json } as Response);
}

function responseError(status: number) {
  return Promise.resolve({
    status,
    text: async () => JSON.stringify({ error: { message: "lookup failed" } }),
  } as Response);
}
