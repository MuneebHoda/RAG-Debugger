import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TracesPage } from "./TracesPage";

const traceId = "018f7a2a-6e2e-7000-a000-000000000201";
const runId = "018f7a2a-6e2e-7000-a000-000000000202";
const sourceId = "018f7a2a-6e2e-7000-a000-000000000203";
const documentId = "018f7a2a-6e2e-7000-a000-000000000204";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000205";
const datasetId = "018f7a2a-6e2e-7000-a000-000000000210";

const source = {
  id: sourceId,
  project_id: "018f7a2a-6e2e-7000-a000-000000000206",
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
  path: "platform-guide.md",
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
};

const retrieval = {
  run: {
    id: runId,
    query: "gpu embedding workers",
    top_k: 5,
    retrieval_mode: "hybrid",
    latency_ms: 8,
    created_at: "2026-06-23T00:00:00Z",
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
      chunk,
      document,
      source,
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
        document_id: documentId,
        document_path: "platform-guide.md",
        chunk_ordinal: 0,
        section_title: "Indexing",
        checksum_prefix: "1234567890ab",
        snippet: "GPU workers speed up embedding refreshes.",
      },
      quality_flags: ["semantic_match"],
      evidence_strength: "strong",
      duplicate_count: 1,
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
};

const trace = {
  id: traceId,
  project_id: source.project_id,
  input: "gpu embedding workers",
  output: "GPU workers speed up embedding refreshes [1]",
  started_at: "2026-06-23T00:00:00Z",
  completed_at: "2026-06-23T00:00:01Z",
  failure_labels: ["weak_evidence"],
  source_run_id: runId,
  summary: "Retrieved one chunk, but CorpusLab found one quality signal.",
  status: "warning",
  evidence_strength: "strong",
  spans: [
    {
      id: "018f7a2a-6e2e-7000-a000-000000000207",
      kind: "query_input",
      title: "Query input",
      description: "Captured query settings.",
      started_at: "2026-06-23T00:00:00Z",
      completed_at: "2026-06-23T00:00:00Z",
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
    {
      id: "018f7a2a-6e2e-7000-a000-000000000208",
      kind: "retrieval",
      title: "Retrieval ranking",
      description: "Scored chunks.",
      started_at: "2026-06-23T00:00:00Z",
      completed_at: "2026-06-23T00:00:01Z",
      latency_ms: 8,
      status: "succeeded",
      detail: {
        type: "retrieval",
        hit_count: 1,
        top_score: 3.4,
        embedding_readiness: "ready",
      },
    },
  ],
  retrieval,
  reruns: [],
};

describe("TracesPage", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = input.toString();
        if (url.endsWith("/api/v1/traces")) {
          return responseJson([
            {
              id: traceId,
              query: "gpu embedding workers",
              retrieval_mode: "hybrid",
              latency_ms: 8,
              evidence_strength: "strong",
              failure_labels: ["weak_evidence"],
              span_count: 2,
              rerun_count: 0,
              created_at: "2026-06-23T00:00:00Z",
            },
          ]);
        }
        if (url.endsWith("/api/v1/eval-lab/datasets")) {
          return responseJson([
            {
              id: datasetId,
              name: "Default retrieval dataset",
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
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}/cases`)) {
          return responseJson({
            id: "018f7a2a-6e2e-7000-a000-000000000211",
            name: "gpu embedding workers",
            query: "gpu embedding workers",
            top_k: 5,
            expected_chunk_ids: [chunkId],
            expected_document_ids: [documentId],
            notes: `Saved from trace ${traceId.slice(0, 8)}.`,
            created_at: "2026-06-23T00:00:00Z",
          });
        }
        if (url.endsWith(`/api/v1/traces/${traceId}/rerun`)) {
          return responseJson({
            trace: {
              ...trace,
              reruns: [
                {
                  id: "018f7a2a-6e2e-7000-a000-000000000209",
                  request: {
                    query: "gpu embedding workers",
                    top_k: 3,
                    retrieval_mode: "lexical",
                    source_ids: [],
                    document_ids: [],
                  },
                  response: {
                    ...retrieval,
                    run: {
                      ...retrieval.run,
                      retrieval_mode: "lexical",
                      top_k: 3,
                    },
                  },
                  score_delta: -0.4,
                  latency_delta_ms: 2,
                  overlap_count: 1,
                  changed_rank_count: 0,
                  created_at: "2026-06-23T00:00:02Z",
                },
              ],
            },
            comparison: {
              id: "018f7a2a-6e2e-7000-a000-000000000209",
              request: {
                query: "gpu embedding workers",
                top_k: 3,
                retrieval_mode: "lexical",
                source_ids: [],
                document_ids: [],
              },
              response: {
                ...retrieval,
                run: {
                  ...retrieval.run,
                  retrieval_mode: "lexical",
                  top_k: 3,
                },
              },
              score_delta: -0.4,
              latency_delta_ms: 2,
              overlap_count: 1,
              changed_rank_count: 0,
              created_at: "2026-06-23T00:00:02Z",
            },
          });
        }
        return responseJson(trace);
      }),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders trace timeline, evidence, explainers, and rerun comparison", async () => {
    render(
      <MemoryRouter>
        <TracesPage />
      </MemoryRouter>,
    );

    expect(
      await screen.findByRole("heading", { name: /trace debugger/i }),
    ).toBeInTheDocument();
    expect(await screen.findByText("Retrieval ranking")).toBeInTheDocument();
    expect(screen.getAllByText(/weak evidence/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/GPU workers speed up/i)).toBeInTheDocument();
    expect(screen.getByText("Failure label")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/^mode$/i), {
      target: { value: "lexical" },
    });
    fireEvent.change(screen.getByLabelText(/top k/i), {
      target: { value: "3" },
    });
    fireEvent.click(screen.getByRole("button", { name: /rerun trace/i }));

    await waitFor(() =>
      expect(screen.getByText("Score delta")).toBeInTheDocument(),
    );
    expect(screen.getByText("-0.40")).toBeInTheDocument();
    expect(screen.getAllByText("1 hits").length).toBeGreaterThan(0);
  });

  it("saves trace evidence into Eval Lab", async () => {
    render(
      <MemoryRouter>
        <TracesPage />
      </MemoryRouter>,
    );

    await screen.findByText("Retrieval ranking");
    fireEvent.click(screen.getByRole("button", { name: /save to eval lab/i }));

    expect(
      await screen.findByText(/saved to default retrieval dataset/i),
    ).toBeInTheDocument();
  });
});

function responseJson(json: unknown) {
  return Promise.resolve({
    status: 200,
    json: async () => json,
  });
}
