import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DatasetDetailPage } from "./DatasetDetailPage";
import { ExperimentDetailPage } from "./ExperimentDetailPage";
import { EvalsPage } from "./EvalsPage";

const datasetId = "018f7a2a-6e2e-7000-a000-000000000301";
const caseId = "018f7a2a-6e2e-7000-a000-000000000302";
const sourceId = "018f7a2a-6e2e-7000-a000-000000000303";
const documentId = "018f7a2a-6e2e-7000-a000-000000000304";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000305";
const experimentId = "018f7a2a-6e2e-7000-a000-000000000306";

describe("guided Quality workflow", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = input.toString();
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}`)) {
          return responseJson(dataset());
        }
        if (url.endsWith(`/api/v1/eval-lab/experiments/${experimentId}`)) {
          return responseJson(experiment());
        }
        if (url.endsWith("/api/v1/eval-lab/datasets")) {
          return responseJson([datasetSummary()]);
        }
        if (url.endsWith("/api/v1/eval-lab/experiments")) {
          return responseJson([experiment()]);
        }
        if (url.endsWith("/api/v1/eval-lab/ci/runs")) {
          return responseJson([]);
        }
        if (url.endsWith(`/api/v1/documents/${documentId}/chunks`)) {
          return responseJson([chunk()]);
        }
        if (url.endsWith("/api/v1/sources")) {
          return responseJson([sourceSummary()]);
        }
        return responseJson([]);
      }),
    );
  });

  afterEach(() => vi.unstubAllGlobals());

  it("leads with datasets and gate decisions", async () => {
    renderRoute(
      "/app/evals",
      <Route path="/app/evals" element={<EvalsPage />} />,
    );

    expect(
      await screen.findByRole("heading", { name: "Quality" }),
    ).toBeInTheDocument();
    expect(
      await screen.findByText("Production corpus gate"),
    ).toBeInTheDocument();
    expect(await screen.findAllByText("failed")).not.toHaveLength(0);
    expect(screen.getByText(/recent experiments/i)).toBeInTheDocument();
  });

  it("opens a focused dataset with cases and experiment controls", async () => {
    renderRoute(
      `/app/evals/datasets/${datasetId}`,
      <Route
        path="/app/evals/datasets/:datasetId"
        element={<DatasetDetailPage />}
      />,
    );

    expect(
      await screen.findByRole("heading", { name: "Production corpus gate" }),
    ).toBeInTheDocument();
    expect(screen.getByText("GPU indexing evidence")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /run experiment/i }),
    ).toBeEnabled();
    expect(
      screen.getByRole("button", { name: /add case/i }),
    ).toBeInTheDocument();
  });

  it("shows gate failures before the detailed mode metrics", async () => {
    renderRoute(
      `/app/evals/experiments/${experimentId}`,
      <Route
        path="/app/evals/experiments/:experimentId"
        element={<ExperimentDetailPage />}
      />,
    );

    expect(
      await screen.findByRole("heading", { name: "Release retrieval gate" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /gate failed/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/expected evidence was not retrieved/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /mode comparison/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Create audit report" }),
    ).toBeInTheDocument();
  });
});

function renderRoute(initialEntry: string, route: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <Routes>{route}</Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function datasetSummary() {
  return {
    id: datasetId,
    name: "Production corpus gate",
    description: "Critical support and platform questions.",
    case_count: 1,
    latest_experiment_id: experimentId,
    latest_gate: gate("failed"),
    latest_average_recall_at_k: 0.5,
    latest_average_precision_at_k: 0.4,
    updated_at: "2026-06-25T00:00:00Z",
  };
}

function dataset() {
  return {
    id: datasetId,
    name: "Production corpus gate",
    description: "Critical support and platform questions.",
    cases: [
      {
        id: caseId,
        name: "GPU indexing evidence",
        query: "Which evidence explains GPU indexing workers?",
        top_k: 5,
        expected_chunk_ids: [chunkId],
        expected_document_ids: [documentId],
        notes: "Required launch-quality evidence.",
        created_at: "2026-06-25T00:00:00Z",
      },
    ],
    created_at: "2026-06-25T00:00:00Z",
    updated_at: "2026-06-25T00:00:00Z",
  };
}

function sourceSummary() {
  return {
    source: {
      id: sourceId,
      project_id: "project-1",
      name: "Platform docs",
      kind: { FileSet: { root_hint: "browser-upload" } },
      sync_policy: "Manual",
      chunking: {
        target_tokens: 512,
        overlap_tokens: 64,
        strategy: "structured",
      },
    },
    document_count: 1,
    chunk_count: 1,
    documents: [
      {
        document: {
          id: documentId,
          source_id: sourceId,
          path: "platform-guide.md",
          mime_type: "text/markdown",
          checksum: "abcdef",
          byte_size: 128,
          profile: "technical_docs",
          extraction_quality: "high",
          warnings: [],
        },
        chunk_count: 1,
      },
    ],
  };
}

function chunk() {
  return {
    id: chunkId,
    document_id: documentId,
    ordinal: 0,
    text: "GPU workers accelerate embedding indexing.",
    token_count: 5,
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
}

function experiment() {
  return {
    id: experimentId,
    dataset_id: datasetId,
    dataset_name: "Production corpus gate",
    name: "Release retrieval gate",
    modes: ["hybrid", "vector", "lexical"],
    top_k: 5,
    config_snapshot: {
      top_k: 5,
      scoring_weights: {},
      embedding_model: {
        provider: "local",
        model_name: "local-hash-v1",
        dimension: 384,
      },
      dataset_case_count: 1,
    },
    mode_results: [
      modeResult("hybrid", 0.5, 0.4, 20),
      modeResult("vector", 0.25, 0.2, 18),
      modeResult("lexical", 0, 0, 12),
    ],
    comparison: {
      best_mode: "hybrid",
      mode_count: 3,
      recall_delta: 0.5,
      precision_delta: 0.4,
      latency_delta_ms: 8,
      summary: "Hybrid leads by recall and precision.",
    },
    gate: gate("failed"),
    failures: [
      {
        case_id: caseId,
        query: "Which evidence explains GPU indexing workers?",
        retrieval_mode: "hybrid",
        label: "expected_evidence_missing",
        severity: "critical",
        message: "Expected evidence was not retrieved.",
        top_hit_rank: null,
      },
    ],
    created_at: "2026-06-25T00:00:00Z",
  };
}

function modeResult(
  mode: string,
  recall: number,
  precision: number,
  latency: number,
) {
  return {
    retrieval_mode: mode,
    case_count: 1,
    passed_count: recall >= 0.8 ? 1 : 0,
    average_recall_at_k: recall,
    average_precision_at_k: precision,
    mean_reciprocal_rank: recall,
    citation_coverage: recall,
    weak_evidence_count: 1,
    missing_embedding_failures: 0,
    latency_p50_ms: latency,
    latency_p95_ms: latency,
    case_results: [],
  };
}

function gate(status: "passed" | "failed") {
  return {
    status,
    average_recall_at_k: 0.5,
    weak_evidence_rate: 1,
    critical_failure_count: status === "failed" ? 1 : 0,
    recall_threshold: 0.8,
    weak_evidence_limit: 0.2,
    reasons:
      status === "failed"
        ? ["Average recall is below 80%."]
        : ["All gate rules passed."],
  };
}

function responseJson(json: unknown) {
  return Promise.resolve({ status: 200, json: async () => json } as Response);
}
