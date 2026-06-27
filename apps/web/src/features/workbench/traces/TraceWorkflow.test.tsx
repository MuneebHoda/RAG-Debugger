import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RunsPage } from "./RunsPage";
import { TraceDetailPage } from "./TraceDetailPage";

const traceId = "018f7a2a-6e2e-7000-a000-000000000201";
const datasetId = "018f7a2a-6e2e-7000-a000-000000000210";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000205";

describe("guided run workflow", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = input.toString();
        if (url.endsWith("/api/v1/traces")) {
          return responseJson([traceSummary]);
        }
        if (url.endsWith(`/api/v1/traces/${traceId}/rerun`)) {
          return responseJson({
            trace: { ...trace, reruns: [comparison] },
            comparison,
          });
        }
        if (url.endsWith("/api/v1/eval-lab/datasets")) {
          return responseJson([
            {
              id: datasetId,
              name: "Critical questions",
              description: null,
              case_count: 0,
              latest_experiment_id: null,
              latest_gate: null,
              latest_average_recall_at_k: null,
              latest_average_precision_at_k: null,
              updated_at: "2026-06-27T10:46:19Z",
            },
          ]);
        }
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}/cases`)) {
          return responseJson({ id: "case-1" });
        }
        return responseJson(trace);
      }),
    );
  });

  afterEach(() => vi.unstubAllGlobals());

  it("lists saved runs and links to focused detail pages", async () => {
    renderWithClient(
      <MemoryRouter>
        <RunsPage />
      </MemoryRouter>,
    );

    expect(await screen.findByText(traceSummary.query)).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /gpu embedding workers/i }),
    ).toHaveAttribute("href", `/app/traces/${traceId}`);
  });

  it("shows diagnosis tabs and comparison controls", async () => {
    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText(/likely causes/i)).toBeInTheDocument();
    expect(
      screen.getByText(/too weak for a confident answer/i),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /evidence/i }));
    expect(screen.getByText(/gpu workers speed up/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /compare/i }));
    fireEvent.change(screen.getByLabelText(/retrieval mode/i), {
      target: { value: "lexical" },
    });
    fireEvent.click(screen.getByRole("button", { name: /run comparison/i }));
    expect(await screen.findByText("Top-score change")).toBeInTheDocument();
  });

  it("requires an explicit dataset and evidence selection for Quality", async () => {
    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await screen.findByText(/likely causes/i);
    fireEvent.click(screen.getByRole("button", { name: /choose evidence/i }));
    const datasetSelect = await screen.findByLabelText(/quality dataset/i);
    await screen.findByRole("option", { name: "Critical questions" });
    expect(datasetSelect).toHaveValue("");
    expect(
      screen.getByRole("button", { name: /save quality case/i }),
    ).toBeDisabled();

    fireEvent.change(datasetSelect, {
      target: { value: datasetId },
    });
    expect(datasetSelect).toHaveValue(datasetId);
    const evidenceCheckbox = screen.getByRole("checkbox");
    fireEvent.click(evidenceCheckbox);
    await waitFor(() => expect(evidenceCheckbox).toBeChecked());
    const saveButton = screen.getByRole("button", {
      name: /save quality case/i,
    });
    expect(saveButton).toBeEnabled();
  });
});

function renderWithClient(children: React.ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>{children}</QueryClientProvider>,
  );
}

const traceSummary = {
  id: traceId,
  query: "gpu embedding workers",
  retrieval_mode: "hybrid",
  latency_ms: 8,
  evidence_strength: "weak",
  failure_labels: ["weak_evidence"],
  span_count: 2,
  rerun_count: 0,
  created_at: "2026-06-27T10:46:19Z",
};

const retrieval = {
  run: {
    id: "run-1",
    query: traceSummary.query,
    top_k: 5,
    retrieval_mode: "hybrid",
    latency_ms: 8,
    created_at: "2026-06-27T10:46:19Z",
  },
  answer: {
    status: "answered",
    text: "GPU workers speed up embedding refreshes [1]",
    citations: [],
  },
  hits: [
    {
      rank: 1,
      score: 3.4,
      chunk: {
        id: chunkId,
        document_id: "document-1",
        ordinal: 0,
        text: "GPU workers speed up embedding refreshes.",
        token_count: 6,
        byte_range: { start: 0, end: 42 },
        checksum: "1234567890abcdef",
        strategy: "structured",
        section_title: "Indexing",
        split_reason: "document_end",
        quality_flags: ["good_evidence_candidate"],
        is_duplicate: false,
        text_density: 0.9,
        evidence_score_hint: 0.8,
      },
      document: {
        id: "document-1",
        source_id: "source-1",
        path: "platform-guide.md",
        mime_type: "text/markdown",
        checksum: "abcdef",
        byte_size: 64,
        profile: "technical_docs",
        extraction_quality: "high",
        warnings: [],
      },
      source: {
        id: "source-1",
        project_id: "project-1",
        name: "Corpus upload",
        kind: { FileSet: { root_hint: "browser-upload" } },
        sync_policy: "Manual",
        chunking: {
          target_tokens: 512,
          overlap_tokens: 64,
          strategy: "structured",
        },
      },
      matched_terms: [{ term: "gpu", count: 1 }],
      score_breakdown: {
        semantic: 0.9,
        lexical: 1.8,
        phrase: 0.4,
        section: 0.1,
        path: 0,
        metadata: 0.1,
      },
      normalized_score_breakdown: {
        semantic: 0.5,
        lexical: 1,
        phrase: 0.2,
        section: 0.05,
        path: 0,
        metadata: 0.05,
      },
      snippet: "GPU workers speed up embedding refreshes.",
      citation: {
        label: "[1]",
        chunk_id: chunkId,
        document_id: "document-1",
        document_path: "platform-guide.md",
        chunk_ordinal: 0,
        section_title: "Indexing",
        checksum_prefix: "1234567890ab",
        snippet: "GPU workers speed up embedding refreshes.",
      },
      quality_flags: ["semantic_match"],
      evidence_strength: "weak",
      duplicate_count: 1,
    },
  ],
  embedding_status: {
    readiness: "ready",
    required: true,
    model: { provider: "local", model_name: "local-hash-v1", dimension: 384 },
    total_chunks: 1,
    indexed_chunks: 1,
    missing_chunks: 0,
    stale_chunks: 0,
  },
};

const trace = {
  id: traceId,
  project_id: "project-1",
  input: traceSummary.query,
  output: retrieval.answer.text,
  started_at: "2026-06-27T10:46:19Z",
  completed_at: "2026-06-27T10:46:20Z",
  failure_labels: ["weak_evidence"],
  source_run_id: "run-1",
  summary: "Retrieved one chunk, but the evidence is weak.",
  status: "warning",
  evidence_strength: "weak",
  spans: [
    {
      id: "span-1",
      kind: "query_input",
      title: "Query input",
      description: "Captured the question.",
      started_at: "2026-06-27T10:46:19Z",
      completed_at: "2026-06-27T10:46:19Z",
      latency_ms: 0,
      status: "succeeded",
      detail: {
        type: "query_input",
        top_k: 5,
        retrieval_mode: "hybrid",
        source_filter_count: 0,
        document_filter_count: 0,
      },
    },
  ],
  retrieval,
  reruns: [],
};

const comparison = {
  id: "comparison-1",
  request: {
    query: traceSummary.query,
    top_k: 5,
    retrieval_mode: "lexical",
    source_ids: [],
    document_ids: [],
  },
  response: {
    ...retrieval,
    run: { ...retrieval.run, retrieval_mode: "lexical" },
  },
  score_delta: -0.4,
  latency_delta_ms: 2,
  overlap_count: 1,
  changed_rank_count: 0,
  created_at: "2026-06-27T10:46:20Z",
};

function responseJson(json: unknown) {
  return Promise.resolve({
    status: 200,
    json: async () => json,
  } as Response);
}
