import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
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
const baselineId = "018f7a2a-6e2e-7000-a000-000000000307";
const partialBaselineId = "018f7a2a-6e2e-7000-a000-000000000308";
const newerExperimentId = "018f7a2a-6e2e-7000-a000-000000000309";
const firstExperimentId = "018f7a2a-6e2e-7000-a000-000000000310";

describe("guided Eval Lab workflow", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = input.toString();
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}`)) {
          return responseJson(dataset());
        }
        if (
          url.includes(`/api/v1/eval-lab/datasets/${datasetId}/experiments`)
        ) {
          return responseJson(experimentHistory());
        }
        if (url.includes(`/api/v1/eval-lab/datasets/${datasetId}/trends`)) {
          return responseJson(trendSummary());
        }
        if (url.endsWith(`/api/v1/eval-lab/experiments/${experimentId}`)) {
          return responseJson(experiment());
        }
        if (url.endsWith(`/api/v1/eval-lab/experiments/${firstExperimentId}`)) {
          return responseJson(firstExperiment());
        }
        if (
          url.includes(
            `/api/v1/eval-lab/experiments/${experimentId}/regression`,
          )
        ) {
          return responseJson(regressionForUrl(url));
        }
        if (
          url.includes(
            `/api/v1/eval-lab/experiments/${firstExperimentId}/regression`,
          )
        ) {
          return responseJson(noBaselineRegression());
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
      await screen.findByRole("heading", { name: "Eval Lab" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/eval lab is the measurement step/i),
    ).toBeInTheDocument();
    expect(
      await screen.findByText("Production corpus gate"),
    ).toBeInTheDocument();
    expect(await screen.findAllByText("failed")).not.toHaveLength(0);
    expect(screen.getByText(/quality trend/i)).toBeInTheDocument();
    expect(screen.getAllByText(/regressed/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/recent experiments/i)).toBeInTheDocument();
  });

  it("exposes CI Runs as a focused quality view", async () => {
    renderRoute(
      "/app/evals?view=ci-runs",
      <Route path="/app/evals" element={<EvalsPage />} />,
    );

    expect(
      await screen.findByRole("heading", { name: "CI Runs" }),
    ).toBeInTheDocument();
    expect(await screen.findByText("No CI quality runs")).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /Manage API keys/ }),
    ).toHaveAttribute("href", "/app/settings?tab=api-keys");
    expect(
      screen.getByRole("link", { name: /Open API key setup/ }),
    ).toHaveAttribute("href", "/app/settings?tab=api-keys");
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
    expect(screen.getByText(/experiment history/i)).toBeInTheDocument();
    expect(
      screen.getAllByText(/release retrieval gate/i).length,
    ).toBeGreaterThan(0);
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
      screen.getByRole("heading", { name: /regression history/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("combobox", { name: "Baseline experiment" }),
    ).toHaveValue("auto");
    expect(
      await screen.findByText(/Automatic: Baseline retrieval gate/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/newly failed/i)).toBeInTheDocument();
    expect(screen.getByText(/changed top evidence/i)).toBeInTheDocument();
    expect(screen.getByText(/changed failure labels/i)).toBeInTheDocument();
    expect(
      screen.getByText(/Classification is regressed/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /mode comparison/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Create audit report" }),
    ).toBeInTheDocument();
  });

  it("persists explicit baseline choice and warns for partial compatibility", async () => {
    renderRoute(
      `/app/evals/experiments/${experimentId}?baseline_id=${partialBaselineId}`,
      <Route
        path="/app/evals/experiments/:experimentId"
        element={<ExperimentDetailPage />}
      />,
    );

    const selector = await screen.findByRole("combobox", {
      name: "Baseline experiment",
    });
    expect(await screen.findByText(/Partial baseline/i)).toBeInTheDocument();
    expect(selector).toHaveValue(partialBaselineId);
    expect(
      await screen.findByText(/top_k changed from 10 to 5/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Partial baseline comparison is unchanged/i),
    ).toBeInTheDocument();

    fireEvent.change(selector, { target: { value: baselineId } });

    expect(selector).toHaveValue(baselineId);
    expect(
      (await screen.findAllByText(/Baseline retrieval gate/i)).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText(/Partial baseline/i)).not.toBeInTheDocument();
  });

  it("shows incompatible baseline options but does not allow selecting them", async () => {
    renderRoute(
      `/app/evals/experiments/${experimentId}`,
      <Route
        path="/app/evals/experiments/:experimentId"
        element={<ExperimentDetailPage />}
      />,
    );

    await screen.findByRole("option", {
      name: /Release retrieval gate · incompatible/i,
    });
    expect(
      screen.getByRole("option", {
        name: /Release retrieval gate · incompatible/i,
      }),
    ).toBeDisabled();
    expect(
      screen.getByRole("option", {
        name: /Future retrieval gate · incompatible/i,
      }),
    ).toBeDisabled();
  });

  it("distinguishes no baseline from unchanged regression", async () => {
    renderRoute(
      `/app/evals/experiments/${firstExperimentId}`,
      <Route
        path="/app/evals/experiments/:experimentId"
        element={<ExperimentDetailPage />}
      />,
    );

    expect(
      await screen.findByRole("heading", { name: "Initial retrieval gate" }),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText(/No comparable baseline/i).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getByText(/Run another compatible experiment/i),
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

function experimentHistory() {
  return [
    experimentSummary({
      id: newerExperimentId,
      name: "Future retrieval gate",
      created_at: "2026-06-27T00:00:00Z",
      gate_status: "passed",
    }),
    experimentSummary(),
    experimentSummary({
      id: partialBaselineId,
      name: "Partial retrieval gate",
      modes: ["hybrid"],
      top_k: 10,
      created_at: "2026-06-24T00:00:00Z",
      gate_status: "passed",
    }),
    experimentSummary({
      id: baselineId,
      name: "Baseline retrieval gate",
      created_at: "2026-06-23T00:00:00Z",
      gate_status: "passed",
    }),
  ];
}

function experimentSummary(
  overrides: Partial<ReturnType<typeof baseExperimentSummary>> = {},
) {
  return { ...baseExperimentSummary(), ...overrides };
}

function baseExperimentSummary() {
  return {
    id: experimentId,
    dataset_id: datasetId,
    dataset_name: "Production corpus gate",
    name: "Release retrieval gate",
    modes: ["hybrid", "vector", "lexical"],
    top_k: 5,
    best_mode: "hybrid",
    gate_status: "failed",
    average_recall_at_k: 0.5,
    average_precision_at_k: 0.4,
    mean_reciprocal_rank: 0.5,
    citation_coverage: 0.5,
    weak_evidence_case_rate: 1,
    missing_embedding_failures: 0,
    latency_p50_ms: 20,
    latency_p95_ms: 20,
    failure_count: 1,
    created_at: "2026-06-25T00:00:00Z",
  };
}

function trendSummary() {
  return {
    dataset_id: datasetId,
    experiment_count: 2,
    window_limit: 10,
    latest_experiment_id: experimentId,
    latest_gate_status: "failed",
    points: [experimentSummary()],
    latest_regression: regression(),
  };
}

function regression() {
  return {
    current_experiment_id: experimentId,
    baseline_experiment_id: baselineId,
    classification: "regressed",
    current_gate_status: "failed",
    baseline_gate_status: "passed",
    metric_deltas: [
      {
        metric: "recall_at_k",
        current: 0.5,
        baseline: 1,
        delta: -0.5,
        classification: "regressed",
      },
      {
        metric: "precision_at_k",
        current: 0.4,
        baseline: 1,
        delta: -0.6,
        classification: "regressed",
      },
    ],
    newly_failed_cases: [
      {
        case_id: caseId,
        retrieval_mode: "hybrid",
        query: "Which evidence explains GPU indexing workers?",
        classification: "regressed",
        current_passed: false,
        baseline_passed: true,
        current_top_hit_rank: null,
        baseline_top_hit_rank: 1,
        current_retrieved_chunk_ids: [],
        baseline_retrieved_chunk_ids: [chunkId],
        current_failure_labels: ["expected_evidence_missing"],
        baseline_failure_labels: [],
      },
    ],
    recovered_cases: [],
    changed_top_evidence_cases: [
      {
        case_id: caseId,
        retrieval_mode: "hybrid",
        query: "Which evidence explains GPU indexing workers?",
        classification: "regressed",
        current_passed: false,
        baseline_passed: true,
        current_top_hit_rank: 4,
        baseline_top_hit_rank: 1,
        current_retrieved_chunk_ids: ["018f7a2a-6e2e-7000-a000-000000000399"],
        baseline_retrieved_chunk_ids: [chunkId],
        current_failure_labels: ["expected_evidence_missing"],
        baseline_failure_labels: [],
      },
    ],
    changed_failure_label_cases: [
      {
        case_id: caseId,
        retrieval_mode: "hybrid",
        query: "Which evidence explains GPU indexing workers?",
        classification: "regressed",
        current_passed: false,
        baseline_passed: true,
        current_top_hit_rank: null,
        baseline_top_hit_rank: 1,
        current_retrieved_chunk_ids: [],
        baseline_retrieved_chunk_ids: [chunkId],
        current_failure_labels: ["expected_evidence_missing"],
        baseline_failure_labels: ["low_precision"],
      },
    ],
    summary: "Release retrieval gate regressed compared with Baseline.",
  };
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

function firstExperiment() {
  return {
    ...experiment(),
    id: firstExperimentId,
    name: "Initial retrieval gate",
    created_at: "2026-06-22T00:00:00Z",
    gate: gate("passed"),
    failures: [],
  };
}

function regressionForUrl(url: string) {
  const baselineIdParam = new URL(url, "http://127.0.0.1").searchParams.get(
    "baseline_id",
  );
  if (baselineIdParam === partialBaselineId) {
    return {
      ...regression(),
      baseline_experiment_id: partialBaselineId,
      classification: "unchanged",
      newly_failed_cases: [],
      recovered_cases: [],
      changed_top_evidence_cases: [],
      changed_failure_label_cases: [],
      summary: "Partial baseline comparison is unchanged.",
    };
  }
  return regression();
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

function noBaselineRegression() {
  return {
    current_experiment_id: firstExperimentId,
    baseline_experiment_id: null,
    classification: "unchanged",
    current_gate_status: "passed",
    baseline_gate_status: null,
    metric_deltas: [
      {
        metric: "recall_at_k",
        current: 1,
        baseline: null,
        delta: 0,
        classification: "unchanged",
      },
    ],
    newly_failed_cases: [],
    recovered_cases: [],
    changed_top_evidence_cases: [],
    changed_failure_label_cases: [],
    summary: "No comparable baseline exists.",
  };
}

function responseJson(json: unknown) {
  return Promise.resolve({ status: 200, json: async () => json } as Response);
}
