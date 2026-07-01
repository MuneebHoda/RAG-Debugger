import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { RetrievalQueryResponse } from "../../../lib/api/retrieval";
import { RetrievalPage } from "./RetrievalPage";
import { AnswerPanel } from "./RetrievalResults";

const sourceId = "018f7a2a-6e2e-7000-a000-000000000101";
const documentId = "018f7a2a-6e2e-7000-a000-000000000102";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000103";

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
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
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
    render(
      <MemoryRouter>
        <RetrievalPage />
      </MemoryRouter>,
    );

    expect(
      await screen.findByRole("heading", { name: /test retrieval/i }),
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
    render(
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
    render(
      <MemoryRouter>
        <RetrievalPage />
      </MemoryRouter>,
    );

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
    render(
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
});

function responseJson(json: unknown) {
  return Promise.resolve({
    status: 200,
    json: async () => json,
  });
}
