import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { RetrievalQueryResponse } from "../../../lib/api/retrieval";
import { RetrievalPage } from "./RetrievalPage";
import { AnswerPanel } from "./RetrievalResults";

const sourceId = "018f7a2a-6e2e-7000-a000-000000000101";
const documentId = "018f7a2a-6e2e-7000-a000-000000000102";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000103";
const secondDocumentId = "018f7a2a-6e2e-7000-a000-000000000112";
const secondChunkId = "018f7a2a-6e2e-7000-a000-000000000113";
const secondRunId = "018f7a2a-6e2e-7000-a000-000000000115";
const datasetId = "018f7a2a-6e2e-7000-a000-000000000107";
let createdCaseBody: unknown;

const source = {
  id: sourceId,
  project_id: "018f7a2a-6e2e-7000-a000-000000000104",
  name: "Corpus upload",
  kind: { FileSet: { root_hint: "browser-upload" } },
  sync_policy: "Manual",
  chunking: {
    target_tokens: 512,
    overlap_tokens: 64,
    strategy: "structured",
  },
};

const document = {
  id: documentId,
  source_id: sourceId,
  path: "resume.md",
  mime_type: "text/markdown",
  checksum: "abcdef",
  byte_size: 64,
  profile: "technical_docs",
  extraction_quality: "high",
  warnings: [],
};

const chunk = {
  id: chunkId,
  document_id: documentId,
  ordinal: 0,
  text: "Built GPU indexing experiments.",
  token_count: 4,
  byte_range: { start: 0, end: 30 },
  checksum: "1234567890abcdef",
  strategy: "structured",
  section_title: "Projects",
  split_reason: "document_end",
  quality_flags: ["good_evidence_candidate"],
  is_duplicate: false,
  text_density: 0.9,
  evidence_score_hint: 0.8,
};

describe("RetrievalPage", () => {
  beforeEach(() => {
    createdCaseBody = undefined;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = input.toString();
        if (url.endsWith("/api/v1/sources")) {
          return responseJson([
            {
              source,
              document_count: 1,
              chunk_count: 1,
              documents: [{ document, chunk_count: 1 }],
            },
          ]);
        }
        if (url.endsWith("/api/v1/embeddings/status")) {
          return responseJson({
            model: {
              provider: "local",
              model_name: "local-hash-v1",
              dimension: 384,
            },
            total_chunks: 1,
            indexed_chunks: 1,
            missing_chunks: 0,
            stale_chunks: 0,
            last_indexed_at: "2026-06-23T00:00:00Z",
          });
        }
        if (url.endsWith("/api/v1/demo")) {
          return responseJson({
            version: "corpuslab-guided-demo-v1",
            project_id: source.project_id,
            source_id: sourceId,
            progress: {
              sample_corpus_loaded: true,
              chunks_created: true,
              embeddings_indexed: true,
              document_count: 3,
              chunk_count: 12,
              indexed_chunk_count: 12,
              retrieval_run_id: null,
              trace_id: null,
              report_id: null,
            },
            suggested_queries: [
              {
                id: "account_recovery",
                question: "How long is the password reset link valid?",
                description: "Diagnose duplicated support evidence.",
                recommended: true,
              },
            ],
          });
        }
        if (url.endsWith("/api/v1/traces/from-retrieval-run")) {
          return responseJson({
            id: "018f7a2a-6e2e-7000-a000-000000000106",
            project_id: source.project_id,
            input: "gpu indexing",
            output: "Built GPU indexing experiments [1]",
            started_at: "2026-06-23T00:00:00Z",
            completed_at: "2026-06-23T00:00:01Z",
            failure_labels: [],
            source_run_id: "018f7a2a-6e2e-7000-a000-000000000105",
            summary: "Retrieved one strong evidence chunk.",
            status: "completed",
            evidence_strength: "strong",
            spans: [],
            retrieval: null,
            reruns: [],
          });
        }
        if (url.endsWith("/api/v1/eval-lab/datasets")) {
          return responseJson([
            {
              id: datasetId,
              name: "Critical retrieval questions",
              description: null,
              case_count: 0,
              latest_experiment_id: null,
              latest_gate: null,
              latest_average_recall_at_k: null,
              latest_average_precision_at_k: null,
              updated_at: "2026-06-23T00:00:00Z",
            },
          ]);
        }
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}`)) {
          return responseJson({
            id: datasetId,
            name: "Critical retrieval questions",
            description: null,
            cases: [],
            created_at: "2026-06-23T00:00:00Z",
            updated_at: "2026-06-23T00:00:00Z",
          });
        }
        if (url.endsWith("/api/v1/eval-lab/evidence/query")) {
          return responseJson({
            documents: [
              {
                id: documentId,
                source_id: sourceId,
                source_name: "Corpus upload",
                path: "resume.md",
                profile: "technical_docs",
                extraction_quality: "high",
                warnings: [],
                chunk_count: 1,
              },
            ],
            chunks: [
              {
                id: chunkId,
                document_id: documentId,
                source_id: sourceId,
                source_name: "Corpus upload",
                document_path: "resume.md",
                ordinal: 0,
                text: chunk.text,
                token_count: chunk.token_count,
                checksum: chunk.checksum,
                section_title: chunk.section_title,
                quality_flags: chunk.quality_flags,
                is_duplicate: chunk.is_duplicate,
                text_density: chunk.text_density,
                evidence_score_hint: chunk.evidence_score_hint,
              },
            ],
            unresolved_document_ids: [],
            unresolved_chunk_ids: [],
          });
        }
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}/cases`)) {
          createdCaseBody = JSON.parse(String(init?.body ?? "{}"));
          return responseJson({ id: "case-1" });
        }

        return responseJson({
          run: {
            id: "018f7a2a-6e2e-7000-a000-000000000105",
            query: "gpu indexing",
            top_k: 5,
            retrieval_mode: "hybrid",
            latency_ms: 3,
            created_at: "2026-06-23T00:00:00Z",
          },
          answer: {
            status: "answered",
            text: "Built GPU indexing experiments [1]",
            citations: [
              {
                label: "[1]",
                chunk_id: chunkId,
                document_id: documentId,
                document_path: "resume.md",
                chunk_ordinal: 0,
                section_title: "Projects",
                checksum_prefix: "1234567890ab",
                snippet: "Built GPU indexing experiments",
              },
            ],
          },
          hits: [
            {
              rank: 1,
              score: 3.2,
              chunk,
              document,
              source,
              matched_terms: [
                { term: "gpu", count: 1 },
                { term: "indexing", count: 1 },
              ],
              score_breakdown: {
                semantic: 0.7,
                lexical: 2.5,
                phrase: 0.5,
                section: 0,
                path: 0,
                metadata: 0.1,
              },
              normalized_score_breakdown: {
                semantic: 0.28,
                lexical: 1,
                phrase: 0.2,
                section: 0,
                path: 0,
                metadata: 0.04,
              },
              snippet: "Built GPU indexing experiments",
              citation: {
                label: "[1]",
                chunk_id: chunkId,
                document_id: documentId,
                document_path: "resume.md",
                chunk_ordinal: 0,
                section_title: "Projects",
                checksum_prefix: "1234567890ab",
                snippet: "Built GPU indexing experiments",
              },
              quality_flags: ["semantic_match", "exact_term_match"],
              evidence_strength: "strong",
              duplicate_count: 1,
              answer_support: {
                status: "supported",
                reason: "direct_body_support",
                matched_body_term_count: 2,
                query_term_count: 2,
                body_term_coverage: 1,
              },
            },
          ],
          embedding_status: {
            readiness: "ready",
            required: true,
            model: {
              provider: "local",
              model_name: "local-hash-v1",
              dimension: 384,
            },
            total_chunks: 1,
            indexed_chunks: 1,
            missing_chunks: 0,
            stale_chunks: 0,
          },
        });
      }),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders retrieval controls", async () => {
    renderWithClient(<RetrievalPage />);

    expect(
      await screen.findByRole("heading", { name: /^retrieval$/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/retrieval is the test step/i)).toBeInTheDocument();
    expect(
      screen.getByText(/combines semantic and lexical signals/i),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText(/what should the corpus answer/i),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/retrieval mode/i)).toBeInTheDocument();
    expect(screen.getByText(/^advanced$/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /run retrieval/i }),
    ).toBeDisabled();
  });

  it("resolves a guided query id and preselects the demo source", async () => {
    renderWithClient(
      <MemoryRouter
        initialEntries={["/app/retrieval?demo_query=account_recovery"]}
      >
        <RetrievalPage />
      </MemoryRouter>,
    );

    expect(
      await screen.findByDisplayValue(
        "How long is the password reset link valid?",
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByText("Advanced"));
    expect(
      screen.getByRole("checkbox", { name: /corpus upload/i }),
    ).toBeChecked();
  });

  it("submits a query and renders cited evidence", async () => {
    renderWithClient(<RetrievalPage />);

    fireEvent.change(
      await screen.findByLabelText(/what should the corpus answer/i),
      {
        target: { value: "gpu indexing" },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: /run retrieval/i }));

    await waitFor(() =>
      expect(
        screen.getByText("Built GPU indexing experiments [1]"),
      ).toBeInTheDocument(),
    );
    expect(screen.getByText(/\[1\] resume\.md/)).toBeInTheDocument();
    expect(screen.getByText(/gpu × 1/i)).toBeInTheDocument();
    expect(screen.getByText(/Strong · 3\.20/i)).toBeInTheDocument();
    expect(screen.getByText(/Exact term/i)).toBeInTheDocument();
    expect(
      screen.getByText(/answered from chunk body evidence/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/supports answer/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/score breakdown/i)).toBeInTheDocument();
  });

  it("makes an answerability abstention explicit without citations", () => {
    const response = {
      run: {
        id: "run-unsupported",
        query: "unsupported question",
        top_k: 5,
        retrieval_mode: "hybrid",
        latency_ms: 4,
        created_at: "2026-07-01T00:00:00Z",
      },
      answer: {
        status: "insufficient_evidence",
        text: "Ranked candidates were found, but no chunk body directly supports this question.",
        citations: [],
      },
      hits: [],
      embedding_status: {
        readiness: "ready",
        required: true,
        model: {
          provider: "local",
          model_name: "local-hash-v1",
          dimension: 384,
        },
        total_chunks: 1,
        indexed_chunks: 1,
        missing_chunks: 0,
        stale_chunks: 0,
      },
      diagnosis: null,
    } satisfies RetrievalQueryResponse;

    render(
      <AnswerPanel
        isQuerying={false}
        isSavingTrace={false}
        response={response}
        onSaveTrace={vi.fn()}
      />,
    );

    expect(screen.getByText(/^insufficient evidence$/i)).toBeInTheDocument();
    expect(screen.getByText(/none can be cited/i)).toBeInTheDocument();
    expect(screen.queryByText(/^\[1\]/)).not.toBeInTheDocument();
  });

  it("saves the latest retrieval response and opens its debugger", async () => {
    renderWithClient(
      <MemoryRouter initialEntries={["/app/retrieval"]}>
        <Routes>
          <Route path="/app/retrieval" element={<RetrievalPage />} />
          <Route
            path="/app/traces/:traceId"
            element={<h1>Focused run debugger</h1>}
          />
        </Routes>
      </MemoryRouter>,
    );

    fireEvent.change(
      await screen.findByLabelText(/what should the corpus answer/i),
      { target: { value: "gpu indexing" } },
    );
    fireEvent.click(screen.getByRole("button", { name: /run retrieval/i }));

    await screen.findByText("Built GPU indexing experiments [1]");
    fireEvent.click(screen.getByRole("button", { name: /debug this run/i }));

    expect(
      await screen.findByRole("heading", { name: /focused run debugger/i }),
    ).toBeInTheDocument();
  });

  it("saves selected exact evidence to Quality without broadening to the parent document", async () => {
    renderWithClient(<RetrievalPage />);

    fireEvent.change(
      await screen.findByLabelText(/what should the corpus answer/i),
      { target: { value: "gpu indexing" } },
    );
    fireEvent.click(screen.getByRole("button", { name: /run retrieval/i }));
    await screen.findByText("Built GPU indexing experiments [1]");

    fireEvent.click(screen.getByRole("button", { name: /choose evidence/i }));
    const datasetSelect = await screen.findByLabelText(/quality dataset/i);
    await screen.findByRole("option", {
      name: "Critical retrieval questions",
    });
    fireEvent.change(datasetSelect, {
      target: { value: datasetId },
    });
    await waitFor(() => expect(datasetSelect).toHaveValue(datasetId));
    const retrievedEvidenceSection = screen
      .getByRole("heading", { name: "Retrieved evidence from this run" })
      .closest("div");
    expect(retrievedEvidenceSection).not.toBeNull();
    fireEvent.click(
      within(retrievedEvidenceSection!).getByRole("button", {
        name: "Expect this exact chunk",
      }),
    );
    expect(
      await screen.findByText(/Exact chunk expectation/i),
    ).toBeInTheDocument();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /save quality case/i }),
      ).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: /save quality case/i }));

    await waitFor(() => expect(createdCaseBody).toBeDefined());
    expect(createdCaseBody).toMatchObject({
      expected_chunk_ids: [chunkId],
      expected_document_ids: [],
    });
  });

  it("saves only the latest run after a sequential retrieval transition", async () => {
    const baseFetch = vi.mocked(fetch);
    const secondResponse = deferred<Response>();
    let firstResponse: RetrievalQueryResponse | null = null;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = input.toString();
        const body = requestBody(init);
        if (
          url.endsWith("/api/v1/retrieval/query") &&
          body.query === "beta retrieval"
        ) {
          return secondResponse.promise;
        }
        if (url.endsWith("/api/v1/retrieval/query")) {
          const response = await baseFetch(input, init);
          firstResponse = (await response.json()) as RetrievalQueryResponse;
          return responseJson(firstResponse);
        }
        if (
          url.endsWith("/api/v1/eval-lab/evidence/query") &&
          (body.query === "beta retrieval" ||
            (body.chunk_ids as string[] | undefined)?.includes(secondChunkId))
        ) {
          return responseJson(secondEvidenceLookup());
        }
        return baseFetch(input, init);
      }),
    );

    renderWithClient(<RetrievalPage />);
    const question = await screen.findByLabelText(
      /what should the corpus answer/i,
    );
    fireEvent.change(question, { target: { value: "gpu indexing" } });
    fireEvent.click(screen.getByRole("button", { name: /run retrieval/i }));
    await screen.findByText("Built GPU indexing experiments [1]");
    fireEvent.click(screen.getByRole("button", { name: /choose evidence/i }));
    const datasetSelect = await screen.findByLabelText(/quality dataset/i);
    await screen.findByRole("option", {
      name: "Critical retrieval questions",
    });
    fireEvent.change(datasetSelect, { target: { value: datasetId } });
    fireEvent.change(screen.getByLabelText("Case name"), {
      target: { value: "Run A edited name" },
    });
    fireEvent.change(screen.getByLabelText("Notes"), {
      target: { value: "Run A private note" },
    });
    selectCandidateEvidence("Expect this exact chunk");
    selectCandidateEvidence("Accept evidence from this document");

    fireEvent.change(question, { target: { value: "beta retrieval" } });
    fireEvent.click(screen.getByText("Advanced"));
    fireEvent.change(screen.getByLabelText("Results to return"), {
      target: { value: "9" },
    });
    fireEvent.click(screen.getByRole("button", { name: /run retrieval/i }));
    expect(
      await screen.findByText(/saving this previous result is paused/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Save quality case" }),
    ).toBeDisabled();

    expect(firstResponse).not.toBeNull();
    secondResponse.resolve(
      await responseJson(secondRetrievalResponse(firstResponse!)),
    );
    await screen.findByText("Beta retrieval evidence [1]");
    expect(datasetSelect).toHaveValue(datasetId);
    expect(screen.getByLabelText("Case name")).toHaveValue("beta retrieval");
    expect(screen.getByLabelText("Notes")).toHaveValue(
      `Saved from retrieval run ${secondRunId.slice(0, 8)}.`,
    );
    expect(
      screen.getByText(/Select at least one expected document or chunk/i),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Case name"), {
      target: { value: "Run B quality case" },
    });
    fireEvent.change(screen.getByLabelText("Notes"), {
      target: { value: "Run B note" },
    });
    selectCandidateEvidence("Expect this exact chunk");
    selectCandidateEvidence("Accept evidence from this document");
    fireEvent.click(screen.getByRole("button", { name: "Save quality case" }));

    await waitFor(() => expect(createdCaseBody).toBeDefined());
    expect(createdCaseBody).toEqual({
      name: "Run B quality case",
      query: "beta retrieval",
      top_k: 9,
      expected_chunk_ids: [secondChunkId],
      expected_document_ids: [secondDocumentId],
      notes: "Run B note",
    });
    expect(JSON.stringify(createdCaseBody)).not.toContain("Run A");
    expect(JSON.stringify(createdCaseBody)).not.toContain(chunkId);
    expect(JSON.stringify(createdCaseBody)).not.toContain(documentId);
  });
});

function selectCandidateEvidence(name: string) {
  const candidateRegion = screen.getByRole("region", {
    name: "Retrieved evidence from this run",
  });
  fireEvent.click(within(candidateRegion).getByRole("button", { name }));
}

function secondRetrievalResponse(
  template: RetrievalQueryResponse,
): RetrievalQueryResponse {
  const hit = template.hits[0];
  return {
    ...template,
    run: {
      ...template.run,
      id: secondRunId,
      query: "beta retrieval",
      top_k: 9,
    },
    answer: {
      ...template.answer,
      text: "Beta retrieval evidence [1]",
      citations: [],
    },
    hits: [
      {
        ...hit,
        chunk: {
          ...hit.chunk,
          id: secondChunkId,
          document_id: secondDocumentId,
          text: "Beta retrieval evidence.",
        },
        document: {
          ...hit.document,
          id: secondDocumentId,
          path: "beta.md",
        },
        snippet: "Beta retrieval evidence.",
        citation: {
          ...hit.citation,
          chunk_id: secondChunkId,
          document_id: secondDocumentId,
          document_path: "beta.md",
          snippet: "Beta retrieval evidence.",
        },
      },
    ],
  };
}

function secondEvidenceLookup() {
  return {
    documents: [
      {
        id: secondDocumentId,
        source_id: sourceId,
        source_name: "Corpus upload",
        path: "beta.md",
        profile: "technical_docs",
        extraction_quality: "high",
        warnings: [],
        chunk_count: 1,
      },
    ],
    chunks: [
      {
        id: secondChunkId,
        document_id: secondDocumentId,
        source_id: sourceId,
        source_name: "Corpus upload",
        document_path: "beta.md",
        ordinal: 0,
        text_preview: "Beta retrieval evidence.",
        preview_truncated: false,
        token_count: 3,
        checksum: "beta-checksum",
        section_title: "Beta",
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

function requestBody(init?: RequestInit): Record<string, unknown> {
  return typeof init?.body === "string"
    ? (JSON.parse(init.body) as Record<string, unknown>)
    : {};
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function renderWithClient(children: React.ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      {isRouterElement(children) ? (
        children
      ) : (
        <MemoryRouter>{children}</MemoryRouter>
      )}
    </QueryClientProvider>,
  );
}

function isRouterElement(children: React.ReactNode): boolean {
  return (
    typeof children === "object" &&
    children !== null &&
    "type" in children &&
    (children as React.ReactElement).type === MemoryRouter
  );
}

function responseJson(json: unknown): Promise<Response> {
  return Promise.resolve({
    status: 200,
    json: async () => json,
  } as Response);
}
