import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { QueryEvalLabEvidenceResponse } from "../../../../lib/api/evalLab";
import { EvidencePicker } from "./EvidencePicker";
import { EvidenceSelectionReview } from "./EvidenceSelectionReview";
import {
  emptyEvidenceSelection,
  type EvidenceHitRef,
  type EvidenceSelection,
} from "./evidenceSelection";

const documentId = "018f7a2a-6e2e-7000-a000-000000000401";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000402";
const staleDocumentId = "018f7a2a-6e2e-7000-a000-000000000403";
const staleChunkId = "018f7a2a-6e2e-7000-a000-000000000404";
let evidenceResponse: QueryEvalLabEvidenceResponse;

describe("EvidencePicker", () => {
  beforeEach(() => {
    evidenceResponse = resolvedEvidence();
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => responseJson(evidenceResponse)),
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

  it("submits search explicitly and does not request on each keystroke", async () => {
    renderPicker();
    expect(
      await findPreview("GPU workers refresh embeddings."),
    ).toBeInTheDocument();
    const fetchMock = vi.mocked(fetch);
    fetchMock.mockClear();
    const input = screen.getByLabelText("Search corpus evidence");

    fireEvent.change(input, { target: { value: "v" } });
    fireEvent.change(input, { target: { value: "vector" } });
    fireEvent.change(input, { target: { value: "vector index" } });
    expect(fetchMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(requestBody(fetchMock.mock.calls[0][1])).toMatchObject({
      query: "vector index",
      document_limit: 24,
      chunk_limit: 24,
      include_chunks: true,
    });
    expect(fetchMock.mock.calls[0][1]?.signal).toBeInstanceOf(AbortSignal);
  });

  it("submits with Enter and supports bounded empty-query browsing", async () => {
    renderPicker();
    await findPreview("GPU workers refresh embeddings.");
    const fetchMock = vi.mocked(fetch);
    fetchMock.mockClear();
    const input = screen.getByLabelText("Search corpus evidence");

    fireEvent.change(input, { target: { value: "section title" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(requestBody(fetchMock.mock.calls[0][1]).query).toBe("section title");

    fetchMock.mockClear();
    fireEvent.change(input, { target: { value: "" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(requestBody(fetchMock.mock.calls[0][1])).toMatchObject({
      query: "",
      document_limit: 24,
      chunk_limit: 24,
    });
  });

  it("refreshes selected metadata without changing the submitted query", async () => {
    renderPicker();
    const addChunk = await screen.findByRole("button", {
      name: "Expect this exact chunk",
    });
    const fetchMock = vi.mocked(fetch);
    fetchMock.mockClear();

    fireEvent.click(addChunk);

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(requestBody(fetchMock.mock.calls[0][1])).toMatchObject({
      query: "gpu indexing",
      chunk_ids: [chunkId],
    });
  });

  it("keeps newer search results when an older request resolves late", async () => {
    const slow = deferred<Response>();
    let slowSignal: AbortSignal | null = null;
    vi.stubGlobal(
      "fetch",
      vi.fn((_input: RequestInfo | URL, init?: RequestInit) => {
        const body = requestBody(init);
        if (body.query === "slow query") {
          slowSignal = init?.signal ?? null;
          return slow.promise;
        }
        if (body.query === "fast query") {
          return responseJson(evidenceWithPreview("Fast evidence"));
        }
        return responseJson(evidenceWithPreview("Initial evidence"));
      }),
    );
    renderPicker();
    await findPreview("Initial evidence");
    const input = screen.getByLabelText("Search corpus evidence");

    fireEvent.change(input, { target: { value: "slow query" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    await waitFor(() => expect(slowSignal).not.toBeNull());
    fireEvent.change(input, { target: { value: "fast query" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    expect(await findPreview("Fast evidence")).toBeInTheDocument();

    slow.resolve(await responseJson(evidenceWithPreview("Stale evidence")));
    await waitFor(() => expect(slowSignal?.aborted).toBe(true));
    expect(findPreviewNow("Fast evidence")).toBeInTheDocument();
    expect(findPreviewNow("Stale evidence")).not.toBeInTheDocument();
  });

  it("renders bounded previews with an explicit truncation marker", async () => {
    evidenceResponse = evidenceWithPreview("Bounded evidence", true);
    renderPicker();

    expect(await findPreview("Bounded evidence…")).toBeInTheDocument();
  });

  it("keeps stale evidence visible until its accessible controls remove it", async () => {
    evidenceResponse = resolvedEvidence({
      documents: [],
      chunks: [],
      unresolved_document_ids: [staleDocumentId],
      unresolved_chunk_ids: [staleChunkId],
    });
    renderReview({
      documentIds: [staleDocumentId],
      chunkIds: [staleChunkId],
    });

    expect(
      await screen.findByText("Stale/deleted expected document"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Stale/deleted expected chunk"),
    ).toBeInTheDocument();
    expect(requestBody(vi.mocked(fetch).mock.calls[0][1])).toMatchObject({
      document_ids: [staleDocumentId],
      chunk_ids: [staleChunkId],
      document_limit: 0,
      chunk_limit: 0,
    });

    const removeDocument = screen.getByRole("button", {
      name: /Remove stale document/,
    });
    removeDocument.focus();
    expect(removeDocument).toHaveFocus();
    fireEvent.click(removeDocument);
    expect(selectionJson()).toEqual({
      documentIds: [],
      chunkIds: [staleChunkId],
    });

    const removeChunk = await screen.findByRole("button", {
      name: /Remove stale chunk/,
    });
    removeChunk.focus();
    expect(removeChunk).toHaveFocus();
    fireEvent.click(removeChunk);
    expect(selectionJson()).toEqual({ documentIds: [], chunkIds: [] });
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

function renderReview(initialSelection: EvidenceSelection) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <ReviewHarness initialSelection={initialSelection} />
    </QueryClientProvider>,
  );
}

function ReviewHarness({
  initialSelection,
}: {
  initialSelection: EvidenceSelection;
}) {
  const [selection, setSelection] = useState(initialSelection);
  return (
    <>
      <EvidenceSelectionReview
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

function findPreview(text: string) {
  return screen.findByText((_content, element) =>
    isPreviewElement(element, text),
  );
}

function findPreviewNow(text: string) {
  return screen.queryByText((_content, element) =>
    isPreviewElement(element, text),
  );
}

function isPreviewElement(element: Element | null, text: string): boolean {
  return (
    element?.tagName === "SPAN" && element.textContent?.includes(text) === true
  );
}

function responseJson(json: unknown) {
  return Promise.resolve({
    status: 200,
    json: async () => json,
  } as Response);
}

function requestBody(init?: RequestInit): Record<string, unknown> {
  return typeof init?.body === "string"
    ? (JSON.parse(init.body) as Record<string, unknown>)
    : {};
}

function evidenceWithPreview(
  textPreview: string,
  previewTruncated = false,
): QueryEvalLabEvidenceResponse {
  const response = baseEvidence();
  return {
    ...response,
    chunks: response.chunks.map((chunk) => ({
      ...chunk,
      text_preview: textPreview,
      preview_truncated: previewTruncated,
    })),
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function resolvedEvidence(
  overrides: Partial<QueryEvalLabEvidenceResponse> = {},
): QueryEvalLabEvidenceResponse {
  return { ...baseEvidence(), ...overrides };
}

function baseEvidence(): QueryEvalLabEvidenceResponse {
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
      {
        id: chunkId,
        document_id: documentId,
        source_id: "source-1",
        source_name: "Corpus",
        document_path: "platform-guide.md",
        ordinal: 1,
        text_preview: "GPU workers refresh embeddings.",
        preview_truncated: false,
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
  };
}
