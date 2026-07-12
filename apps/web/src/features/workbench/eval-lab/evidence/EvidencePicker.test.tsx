import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EvidencePicker } from "./EvidencePicker";
import {
  emptyEvidenceSelection,
  type EvidenceHitRef,
  type EvidenceSelection,
} from "./evidenceSelection";

const documentId = "018f7a2a-6e2e-7000-a000-000000000401";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000402";

describe("EvidencePicker", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        responseJson({
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
            {
              id: chunkId,
              document_id: documentId,
              source_id: "source-1",
              source_name: "Corpus",
              document_path: "platform-guide.md",
              ordinal: 1,
              text: "GPU workers refresh embeddings.",
              token_count: 5,
              checksum: "abcdef123456",
              section_title: "Indexing",
              quality_flags: [],
              is_duplicate: false,
              text_density: 0.9,
              evidence_score_hint: 0.8,
            },
          ],
          unresolved_document_ids: [],
          unresolved_chunk_ids: [],
        }),
      ),
    );
  });

  afterEach(() => vi.unstubAllGlobals());

  it("keeps exact chunks and document-level expectations independent", async () => {
    renderPicker();

    fireEvent.click(
      await screen.findByRole("button", { name: "Expect this exact chunk" }),
    );

    expect(selectionJson()).toEqual({
      documentIds: [],
      chunkIds: [chunkId],
    });

    fireEvent.click(
      await screen.findByRole("button", {
        name: "Accept evidence from this document",
      }),
    );

    expect(selectionJson()).toEqual({
      documentIds: [documentId],
      chunkIds: [chunkId],
    });
  });

  it("adds retrieved-hit chunks without adding their parent document", () => {
    renderPicker([
      {
        chunkId,
        documentId,
        label: "[1]",
        rank: 1,
        path: "platform-guide.md",
        sectionTitle: "Indexing",
        snippet: "GPU workers refresh embeddings.",
      },
    ]);

    fireEvent.click(
      screen.getAllByRole("button", { name: "Expect this exact chunk" })[0],
    );

    expect(selectionJson()).toEqual({
      documentIds: [],
      chunkIds: [chunkId],
    });
  });
});

function renderPicker(candidateHits: EvidenceHitRef[] = []) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(
    <QueryClientProvider client={client}>
      <PickerHarness candidateHits={candidateHits} />
    </QueryClientProvider>,
  );
}

function PickerHarness({ candidateHits }: { candidateHits: EvidenceHitRef[] }) {
  const [selection, setSelection] = useState<EvidenceSelection>(() =>
    emptyEvidenceSelection(),
  );

  return (
    <>
      <EvidencePicker
        candidateHits={candidateHits}
        query="gpu indexing"
        selection={selection}
        onSelectionChange={setSelection}
      />
      <output aria-label="selection">{JSON.stringify(selection)}</output>
    </>
  );
}

function selectionJson(): EvidenceSelection {
  const output = screen.getByLabelText("selection");
  return JSON.parse(output.textContent ?? "{}") as EvidenceSelection;
}

function responseJson(json: unknown) {
  return Promise.resolve({
    status: 200,
    json: async () => json,
  });
}
