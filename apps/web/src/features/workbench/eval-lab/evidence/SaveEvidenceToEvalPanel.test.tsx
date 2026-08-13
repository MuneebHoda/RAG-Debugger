import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SaveEvidenceToEvalPanel } from "./SaveEvidenceToEvalPanel";
import type { EvidenceHitRef } from "./evidenceSelection";

const datasetA = "018f7a2a-6e2e-7000-a000-000000000701";
const datasetB = "018f7a2a-6e2e-7000-a000-000000000702";
const documentA = "018f7a2a-6e2e-7000-a000-000000000711";
const documentB = "018f7a2a-6e2e-7000-a000-000000000712";
const chunkA = "018f7a2a-6e2e-7000-a000-000000000721";
const chunkB = "018f7a2a-6e2e-7000-a000-000000000722";

describe("SaveEvidenceToEvalPanel", () => {
  let createBodies: Record<string, unknown>[];
  let createResponse: ReturnType<typeof deferred<Response>> | null;
  let createError: Response | null;
  let datasetCases: unknown[];

  beforeEach(() => {
    client = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    createBodies = [];
    createResponse = null;
    createError = null;
    datasetCases = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
        const url = input.toString();
        if (url.endsWith("/api/v1/eval-lab/datasets")) {
          return responseJson([
            datasetSummary(datasetA, "Primary quality"),
            datasetSummary(datasetB, "Secondary quality"),
          ]);
        }
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetA}`)) {
          return responseJson(datasetDetail(datasetA, datasetCases));
        }
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetB}`)) {
          return responseJson(datasetDetail(datasetB, []));
        }
        if (url.endsWith("/api/v1/eval-lab/evidence/query")) {
          return responseJson(evidenceLookup());
        }
        if (url.includes("/cases")) {
          createBodies.push(requestBody(init));
          if (createError) return Promise.resolve(createError);
          if (createResponse) return createResponse.promise;
          return responseJson(savedCase(requestBody(init)));
        }
        throw new Error(`Unexpected request: ${url}`);
      }),
    );
  });

  afterEach(() => vi.unstubAllGlobals());

  it("resets source-owned state while preserving the selected dataset", async () => {
    const view = renderPanel(sourceProps("run-a", "alpha retrieval", 5, hitA));
    const datasetSelect = await selectDataset(datasetA);
    const caseName = screen.getByLabelText("Case name");
    const notes = screen.getByLabelText("Notes");
    fireEvent.change(caseName, { target: { value: "Edited alpha case" } });
    fireEvent.change(notes, { target: { value: "Edited alpha note" } });
    chooseCandidate("Expect this exact chunk");
    chooseCandidate("Accept evidence from this document");
    expect(candidateButton("Expect this exact chunk")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(
      screen.getByRole("button", {
        name: "Clear all selected evidence",
      }),
    );
    expect(candidateButton("Expect this exact chunk")).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    chooseCandidate("Expect this exact chunk");
    chooseCandidate("Accept evidence from this document");

    caseName.focus();
    view.rerender(wrapper(sourceProps("run-b", "beta retrieval", 9, hitB)));

    expect(datasetSelect).toHaveValue(datasetA);
    expect(screen.getByLabelText("Case name")).toHaveValue("beta retrieval");
    expect(screen.getByLabelText("Notes")).toHaveValue(
      "Saved from retrieval run-b.",
    );
    expect(screen.getByLabelText("Case name")).toHaveFocus();
    expect(candidateButton("Expect this exact chunk")).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    expect(
      screen.getByText(/Select at least one expected document or chunk/i),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Case name"), {
      target: { value: "Beta quality case" },
    });
    fireEvent.change(screen.getByLabelText("Notes"), {
      target: { value: "Beta-only note" },
    });
    chooseCandidate("Expect this exact chunk");
    chooseCandidate("Accept evidence from this document");
    fireEvent.click(screen.getByRole("button", { name: "Save quality case" }));

    await waitFor(() => expect(createBodies).toHaveLength(1));
    expect(createBodies[0]).toEqual({
      name: "Beta quality case",
      query: "beta retrieval",
      top_k: 9,
      expected_chunk_ids: [chunkB],
      expected_document_ids: [documentB],
      notes: "Beta-only note",
    });
    expect(JSON.stringify(createBodies[0])).not.toContain("alpha");
    expect(JSON.stringify(createBodies[0])).not.toContain(chunkA);
    expect(JSON.stringify(createBodies[0])).not.toContain(documentA);
  });

  it("pauses the previous source and ignores stale mutation feedback", async () => {
    createResponse = deferred<Response>();
    const view = renderPanel(sourceProps("run-a", "alpha retrieval", 5, hitA));
    await selectDataset(datasetA);
    chooseCandidate("Expect this exact chunk");
    const saveButton = screen.getByRole("button", {
      name: "Save quality case",
    });
    fireEvent.click(saveButton);
    fireEvent.click(saveButton);
    await waitFor(() => expect(createBodies).toHaveLength(1));
    const submissionSignal = vi
      .mocked(fetch)
      .mock.calls.find(([input]) =>
        input.toString().includes("/cases"),
      )?.[1]?.signal;

    view.rerender(
      wrapper({
        ...sourceProps("run-a", "alpha retrieval", 5, hitA),
        sourcePending: true,
      }),
    );
    expect(screen.getByText(/A new retrieval is running/i)).toBeInTheDocument();
    expect(saveButton).toBeDisabled();

    view.rerender(wrapper(sourceProps("run-b", "beta retrieval", 7, hitB)));
    expect(submissionSignal?.aborted).toBe(true);
    createResponse.resolve(await responseJson(savedCase(createBodies[0])));

    await waitFor(() =>
      expect(screen.queryByText("Quality case saved.")).not.toBeInTheDocument(),
    );
    expect(screen.getByLabelText("Case name")).toHaveValue("beta retrieval");
  });

  it("resets save feedback when the dataset changes", async () => {
    renderPanel(sourceProps("run-a", "alpha retrieval", 5, hitA));
    const datasetSelect = await selectDataset(datasetA);
    chooseCandidate("Expect this exact chunk");
    fireEvent.click(screen.getByRole("button", { name: "Save quality case" }));
    expect(await screen.findByText("Quality case saved.")).toBeInTheDocument();

    fireEvent.change(datasetSelect, { target: { value: datasetB } });
    await waitFor(() =>
      expect(screen.queryByText("Quality case saved.")).not.toBeInTheDocument(),
    );
    expect(datasetSelect).toHaveValue(datasetB);
  });

  it("sends trace provenance when saving from a trace", async () => {
    renderPanel({
      ...sourceProps("trace-a", "alpha retrieval", 5, hitA),
      sourceTraceId: "018f7a2a-6e2e-7000-a000-000000000799",
    });
    await selectDataset(datasetA);
    chooseCandidate("Expect this exact chunk");
    fireEvent.click(screen.getByRole("button", { name: "Save quality case" }));

    await waitFor(() => expect(createBodies).toHaveLength(1));
    expect(createBodies[0]).toMatchObject({
      query: "alpha retrieval",
      source_trace_id: "018f7a2a-6e2e-7000-a000-000000000799",
    });
  });

  it("announces dataset loading and failure without enabling save", async () => {
    const datasetsResponse = deferred<Response>();
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        if (input.toString().endsWith("/api/v1/eval-lab/datasets")) {
          return datasetsResponse.promise;
        }
        return responseJson(evidenceLookup());
      }),
    );

    renderPanel(sourceProps("run-a", "alpha retrieval", 5, hitA));

    expect(
      await screen.findByText("Loading Quality datasets…"),
    ).toHaveAttribute("role", "status");
    datasetsResponse.resolve(responseError(503, "Dataset service unavailable"));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Quality datasets could not be loaded",
    );
    expect(
      screen.getByRole("button", { name: "Save quality case" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Retry datasets" }),
    ).toBeInTheDocument();
  });

  it("announces duplicate cases and resets save errors for another dataset", async () => {
    datasetCases = [
      {
        id: "existing-case",
        name: "Existing alpha case",
        query: "  ALPHA   retrieval ",
        top_k: 5,
        expected_chunk_ids: [chunkA],
        expected_document_ids: [],
        notes: null,
        created_at: "2026-07-01T00:00:00Z",
      },
    ];
    createError = responseError(422, "Case could not be saved");
    const view = renderPanel(sourceProps("run-a", "alpha retrieval", 5, hitA));
    const datasetSelect = await selectDataset(datasetA);
    const duplicateWarning = await screen.findByText(
      /Similar case already exists: Existing alpha case/i,
    );
    expect(duplicateWarning).toHaveAttribute("role", "status");
    chooseCandidate("Expect this exact chunk");
    fireEvent.click(screen.getByRole("button", { name: "Save quality case" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Case could not be saved",
    );

    view.rerender(wrapper(sourceProps("run-b", "beta retrieval", 7, hitB)));
    await waitFor(() =>
      expect(
        screen.queryByText("Case could not be saved"),
      ).not.toBeInTheDocument(),
    );
    chooseCandidate("Expect this exact chunk");
    fireEvent.click(screen.getByRole("button", { name: "Save quality case" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Case could not be saved",
    );

    fireEvent.change(datasetSelect, { target: { value: datasetB } });
    await waitFor(() =>
      expect(
        screen.queryByText("Case could not be saved"),
      ).not.toBeInTheDocument(),
    );
  });
});

function renderPanel(
  props: React.ComponentProps<typeof SaveEvidenceToEvalPanel>,
) {
  return render(wrapper(props));
}

function wrapper(props: React.ComponentProps<typeof SaveEvidenceToEvalPanel>) {
  return (
    <QueryClientProvider client={client}>
      <SaveEvidenceToEvalPanel {...props} />
    </QueryClientProvider>
  );
}

let client: QueryClient;

function sourceProps(
  identity: string,
  query: string,
  topK: number,
  hit: EvidenceHitRef,
): React.ComponentProps<typeof SaveEvidenceToEvalPanel> {
  return {
    candidateHits: [hit],
    defaultOpen: true,
    query,
    sourceIdentity: identity,
    sourceNote: `Saved from retrieval ${identity}.`,
    topK,
  };
}

async function selectDataset(datasetId: string) {
  const datasetSelect = await screen.findByLabelText("Quality dataset");
  await screen.findByRole("option", { name: "Primary quality" });
  fireEvent.change(datasetSelect, { target: { value: datasetId } });
  await waitFor(() => expect(datasetSelect).toHaveValue(datasetId));
  return datasetSelect;
}

function candidateButton(name: string) {
  const candidateSection = screen.getByRole("region", {
    name: "Retrieved evidence from this run",
  });
  return within(candidateSection).getByRole("button", { name });
}

function chooseCandidate(name: string) {
  fireEvent.click(candidateButton(name));
}

const hitA: EvidenceHitRef = {
  chunkId: chunkA,
  documentId: documentA,
  label: "[1]",
  path: "alpha.md",
  rank: 1,
  sectionTitle: "Alpha",
  snippet: "Alpha evidence.",
};

const hitB: EvidenceHitRef = {
  chunkId: chunkB,
  documentId: documentB,
  label: "[1]",
  path: "beta.md",
  rank: 1,
  sectionTitle: "Beta",
  snippet: "Beta evidence.",
};

function datasetSummary(id: string, name: string) {
  return {
    id,
    name,
    description: null,
    case_count: 0,
    latest_experiment_id: null,
    latest_gate: null,
    latest_average_recall_at_k: null,
    latest_average_precision_at_k: null,
    updated_at: "2026-07-01T00:00:00Z",
  };
}

function datasetDetail(id: string, cases: unknown[]) {
  return {
    id,
    name: id === datasetA ? "Primary quality" : "Secondary quality",
    description: null,
    cases,
    created_at: "2026-07-01T00:00:00Z",
    updated_at: "2026-07-01T00:00:00Z",
  };
}

function evidenceLookup() {
  return {
    documents: [
      evidenceDocument(documentA, "alpha.md"),
      evidenceDocument(documentB, "beta.md"),
    ],
    chunks: [
      evidenceChunk(chunkA, documentA, "alpha.md"),
      evidenceChunk(chunkB, documentB, "beta.md"),
    ],
    unresolved_document_ids: [],
    unresolved_chunk_ids: [],
  };
}

function evidenceDocument(id: string, path: string) {
  return {
    id,
    source_id: "source-1",
    source_name: "Corpus",
    path,
    profile: "technical_docs",
    extraction_quality: "high",
    warnings: [],
    chunk_count: 1,
  };
}

function evidenceChunk(id: string, documentId: string, path: string) {
  return {
    id,
    document_id: documentId,
    source_id: "source-1",
    source_name: "Corpus",
    document_path: path,
    ordinal: 0,
    text_preview: `${path} evidence`,
    preview_truncated: false,
    token_count: 3,
    checksum: `checksum-${id}`,
    section_title: "Evidence",
    quality_flags: [],
    is_duplicate: false,
    text_density: 0.9,
    evidence_score_hint: 0.8,
  };
}

function savedCase(body: Record<string, unknown>) {
  return {
    id: "case-1",
    name: body.name,
    query: body.query,
    top_k: body.top_k,
    expected_chunk_ids: body.expected_chunk_ids,
    expected_document_ids: body.expected_document_ids,
    notes: body.notes,
    created_at: "2026-07-01T00:00:00Z",
  };
}

function requestBody(init?: RequestInit): Record<string, unknown> {
  return typeof init?.body === "string"
    ? (JSON.parse(init.body) as Record<string, unknown>)
    : {};
}

function responseJson(json: unknown): Promise<Response> {
  return Promise.resolve({
    status: 200,
    json: async () => json,
  } as Response);
}

function responseError(status: number, message: string): Response {
  return {
    status,
    text: async () => JSON.stringify({ error: { message } }),
  } as Response;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}
