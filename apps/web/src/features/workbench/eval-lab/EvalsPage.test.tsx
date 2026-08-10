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

import type {
  RetrievalEvalCase,
  RetrievalEvalRegressionComparison,
} from "../../../lib/api/evalLab";
import { CiRunDetailPage } from "./CiRunDetailPage";
import { DatasetDetailPage } from "./DatasetDetailPage";
import { ExperimentDetailPage } from "./ExperimentDetailPage";
import { EvalsPage } from "./EvalsPage";

const datasetId = "018f7a2a-6e2e-7000-a000-000000000301";
const caseId = "018f7a2a-6e2e-7000-a000-000000000302";
const sourceId = "018f7a2a-6e2e-7000-a000-000000000303";
const documentId = "018f7a2a-6e2e-7000-a000-000000000304";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000305";
const retrievedChunkId = "018f7a2a-6e2e-7000-a000-000000000311";
const experimentId = "018f7a2a-6e2e-7000-a000-000000000306";
const baselineId = "018f7a2a-6e2e-7000-a000-000000000307";
const partialBaselineId = "018f7a2a-6e2e-7000-a000-000000000308";
const newerExperimentId = "018f7a2a-6e2e-7000-a000-000000000309";
const firstExperimentId = "018f7a2a-6e2e-7000-a000-000000000310";
const staleDocumentId = "018f7a2a-6e2e-7000-a000-000000000312";
const staleChunkId = "018f7a2a-6e2e-7000-a000-000000000313";
const ciRunId = "018f7a2a-6e2e-7000-a000-000000000314";
let historyShouldFail = false;
let regressionShouldFail = false;
let ciRunShouldFail = false;
let experimentResponse: ReturnType<typeof experiment> | null = null;
type CiRunFixtureBase = ReturnType<typeof ciRun>;
type CiRunFixture = Omit<
  CiRunFixtureBase,
  | "branch"
  | "commit_sha"
  | "base_ref"
  | "head_ref"
  | "regression"
  | "eval_regression"
> & {
  branch: string | null;
  commit_sha: string | null;
  base_ref: string | null;
  head_ref: string | null;
  regression: CiRunFixtureBase["regression"] | null;
  eval_regression: RetrievalEvalRegressionComparison | null;
};
let ciRunResponse: CiRunFixture | null = null;
let ciRunsResponse: CiRunFixture[] = [];

describe("guided Eval Lab workflow", () => {
  beforeEach(() => {
    historyShouldFail = false;
    regressionShouldFail = false;
    ciRunShouldFail = false;
    experimentResponse = null;
    ciRunResponse = null;
    ciRunsResponse = [];
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
          if (historyShouldFail) {
            return responseError(500, "Experiment history failed");
          }
          return responseJson(experimentHistory());
        }
        if (url.includes(`/api/v1/eval-lab/datasets/${datasetId}/trends`)) {
          return responseJson(trendSummary());
        }
        if (url.endsWith(`/api/v1/eval-lab/experiments/${experimentId}`)) {
          return responseJson(experimentResponse ?? experiment());
        }
        if (url.endsWith(`/api/v1/eval-lab/experiments/${firstExperimentId}`)) {
          return responseJson(firstExperiment());
        }
        if (
          url.includes(
            `/api/v1/eval-lab/experiments/${experimentId}/regression`,
          )
        ) {
          if (regressionShouldFail) {
            return responseError(500, "Regression comparison failed");
          }
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
        if (url.endsWith(`/api/v1/eval-lab/ci/runs/${ciRunId}`)) {
          return ciRunShouldFail
            ? responseError(404, "CI eval run not found")
            : responseJson(ciRunResponse ?? ciRun());
        }
        if (url.endsWith("/api/v1/eval-lab/ci/runs")) {
          return responseJson(ciRunsResponse);
        }
        if (url.endsWith("/api/v1/eval-lab/evidence/query")) {
          return responseJson(evidenceLookup());
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

  it("links saved CI runs to their accessible detail route", async () => {
    ciRunsResponse = [ciRun()];
    renderRoute(
      "/app/evals?view=ci-runs",
      <Route path="/app/evals" element={<EvalsPage />} />,
    );

    expect(
      await screen.findByRole("link", {
        name: "Open CI run for Production corpus gate",
      }),
    ).toHaveAttribute("href", `/app/evals/ci-runs/${ciRunId}`);
    expect(screen.getByText(/feature\/ci-polish · abc123de/i)).toBeVisible();
    expect(screen.getByText(/Config release-v2/i)).toBeVisible();
  });

  it("shows a failed CI gate with metadata, regressions, cases, and report action", async () => {
    const response = ciRun();
    response.eval_regression.metric_deltas.push(
      {
        metric: "latency_p95_ms",
        current: 45,
        baseline: 20,
        delta: 25,
        classification: "regressed",
      },
      {
        metric: "missing_embedding_failures",
        current: 2,
        baseline: 0,
        delta: 2,
        classification: "regressed",
      },
    );
    ciRunResponse = response;
    renderRoute(
      `/app/evals/ci-runs/${ciRunId}`,
      <Route path="/app/evals/ci-runs/:runId" element={<CiRunDetailPage />} />,
    );

    expect(
      await screen.findByRole("heading", { name: "Production corpus gate" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Gate failed" }),
    ).toBeInTheDocument();
    expect(screen.getAllByText("feature/ci-polish")).not.toHaveLength(0);
    expect(screen.getByText("abc123def456")).toBeInTheDocument();
    expect(screen.getByText("release-v2")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Failed metrics" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/recall at k changed from 100% to 50%/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/latency p95 ms changed from 20 ms to 45 ms/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/missing embedding failures changed from 0 to 2/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Regression summary" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Newly failing cases" }),
    ).toBeInTheDocument();
    expect(screen.getByText("No recovered cases.")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Failed cases" }),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText("Expected evidence was not retrieved."),
    ).not.toHaveLength(0);
    expect(
      screen.getByRole("heading", { name: "Metrics summary" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Create audit report" }),
    ).toBeInTheDocument();
  });

  it("keeps inaccessible CI runs inside a recoverable route error", async () => {
    ciRunShouldFail = true;
    renderRoute(
      `/app/evals/ci-runs/${ciRunId}`,
      <Route path="/app/evals/ci-runs/:runId" element={<CiRunDetailPage />} />,
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "This CI run could not be opened.",
    );
    expect(screen.getByRole("button", { name: "Retry" })).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "Back to CI Runs" }),
    ).toHaveAttribute("href", "/app/evals?view=ci-runs");
    ciRunShouldFail = false;
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(
      await screen.findByRole("heading", { name: "Production corpus gate" }),
    ).toBeInTheDocument();
  });

  it("explains a failed metric-only gate without inventing case failures", async () => {
    ciRunResponse = ciRun();
    ciRunResponse.report.failed_cases = [];
    renderRoute(
      `/app/evals/ci-runs/${ciRunId}`,
      <Route path="/app/evals/ci-runs/:runId" element={<CiRunDetailPage />} />,
    );

    expect(
      await screen.findByText(
        "No case-level failures were recorded; review the failed metrics above.",
      ),
    ).toBeInTheDocument();
  });

  it("keeps legacy aggregate regression summaries readable", async () => {
    ciRunResponse = ciRun();
    ciRunResponse.eval_regression = null;
    renderRoute(
      `/app/evals/ci-runs/${ciRunId}`,
      <Route path="/app/evals/ci-runs/:runId" element={<CiRunDetailPage />} />,
    );

    expect(
      await screen.findByText("One case newly failed."),
    ).toBeInTheDocument();
  });

  it("shows a passing run without failure-only actions or a false baseline", async () => {
    ciRunResponse = ciRun();
    ciRunResponse.status = "passed";
    ciRunResponse.gate_status = "passed";
    ciRunResponse.regression = null;
    ciRunResponse.eval_regression = null;
    ciRunResponse.report.gate = gate("passed");
    ciRunResponse.report.failed_cases = [];
    renderRoute(
      `/app/evals/ci-runs/${ciRunId}`,
      <Route path="/app/evals/ci-runs/:runId" element={<CiRunDetailPage />} />,
    );

    expect(
      await screen.findByRole("heading", { name: "Gate passed" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("No comparable CI baseline is available."),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "Failed metrics" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "Failed cases" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Create audit report" }),
    ).not.toBeInTheDocument();
  });

  it("shows a full no-baseline comparison and missing revision metadata", async () => {
    ciRunResponse = ciRun();
    ciRunResponse.status = "passed";
    ciRunResponse.gate_status = "passed";
    ciRunResponse.branch = null;
    ciRunResponse.commit_sha = null;
    ciRunResponse.base_ref = null;
    ciRunResponse.head_ref = null;
    ciRunResponse.regression = null;
    ciRunResponse.eval_regression = noBaselineRegression();
    ciRunResponse.report.gate = gate("passed");
    ciRunResponse.report.failed_cases = [];
    renderRoute(
      `/app/evals/ci-runs/${ciRunId}`,
      <Route path="/app/evals/ci-runs/:runId" element={<CiRunDetailPage />} />,
    );

    expect(await screen.findByText("No baseline")).toBeInTheDocument();
    expect(screen.getByText("No comparable baseline exists.")).toBeVisible();
    expect(screen.getAllByText("Not provided")).toHaveLength(4);
  });

  it("shows recovered cases when a comparable CI run improved", async () => {
    ciRunResponse = ciRun();
    const comparison = regression();
    comparison.classification = "improved";
    comparison.current_gate_status = "passed";
    comparison.baseline_gate_status = "failed";
    comparison.newly_failed_cases = [];
    comparison.recovered_cases = [
      {
        case_id: caseId,
        retrieval_mode: "hybrid",
        query: "Recovered release evidence",
        classification: "improved",
        current_passed: true,
        baseline_passed: false,
        current_top_hit_rank: 1,
        baseline_top_hit_rank: null,
        current_retrieved_chunk_ids: [chunkId],
        baseline_retrieved_chunk_ids: [],
        current_failure_labels: [],
        baseline_failure_labels: ["expected_evidence_missing"],
      },
    ];
    comparison.summary = "The release evidence recovered.";
    ciRunResponse.eval_regression = comparison;
    renderRoute(
      `/app/evals/ci-runs/${ciRunId}`,
      <Route path="/app/evals/ci-runs/:runId" element={<CiRunDetailPage />} />,
    );

    expect(await screen.findByText("improved")).toBeInTheDocument();
    expect(screen.getByText("The release evidence recovered.")).toBeVisible();
    expect(screen.getByText("Recovered release evidence")).toBeVisible();
    expect(screen.getByText(/hybrid · recovered/i)).toBeVisible();
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
    expect(await screen.findByText("Expected exact chunk")).toBeInTheDocument();
    expect(screen.getByText("Expected document")).toBeInTheDocument();
    expect(screen.queryByText("Expected but missing")).not.toBeInTheDocument();
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
      await screen.findByRole("heading", { name: /regression history/i }),
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
    expect(
      await screen.findByText("Expected document retrieved"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Expected document retrieved, wrong chunk"),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/platform-guide\.md/).length).toBeGreaterThan(0);
    expect(screen.queryByText(/metadata unavailable/i)).not.toBeInTheDocument();
  });

  it("shows missing expectations for a completed experiment with zero hits", async () => {
    experimentResponse = experimentWithRetrievedChunks([]);
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
    expect(await screen.findByText("Expected but missing")).toBeInTheDocument();
    expect(screen.getByText("Expected document missing")).toBeInTheDocument();
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
      (await screen.findAllByText(/No comparable baseline/i)).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getByText(/Run another compatible experiment/i),
    ).toBeInTheDocument();
  });

  it("keeps the current experiment identity when experiment history fails", async () => {
    historyShouldFail = true;
    renderRoute(
      `/app/evals/experiments/${experimentId}`,
      <Route
        path="/app/evals/experiments/:experimentId"
        element={<ExperimentDetailPage />}
      />,
    );

    expect(
      await screen.findByText("Experiment history could not be loaded."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /regression history/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/modes hybrid, vector, lexical · top_k 5/i),
    ).toBeInTheDocument();
  });

  it("shows a visible regression error while keeping the selector usable", async () => {
    regressionShouldFail = true;
    renderRoute(
      `/app/evals/experiments/${experimentId}`,
      <Route
        path="/app/evals/experiments/:experimentId"
        element={<ExperimentDetailPage />}
      />,
    );

    expect(
      await screen.findByText("Regression comparison could not be loaded."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("combobox", { name: "Baseline experiment" }),
    ).toBeEnabled();
    expect(screen.getByText("Unavailable")).toBeInTheDocument();
  });

  it("rejects an invalid baseline id from the URL before calling regression", async () => {
    renderRoute(
      `/app/evals/experiments/${experimentId}?baseline_id=${newerExperimentId}`,
      <Route
        path="/app/evals/experiments/:experimentId"
        element={<ExperimentDetailPage />}
      />,
    );

    expect(
      await screen.findByText(
        "Selected baseline cannot be compared with this experiment.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Choose Automatic or a compatible earlier experiment to view regression history.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /use automatic comparison/i }),
    ).toBeInTheDocument();

    const fetchMock = vi.mocked(fetch);
    expect(
      fetchMock.mock.calls.some(([input]) =>
        input
          .toString()
          .includes(
            `/api/v1/eval-lab/experiments/${experimentId}/regression?baseline_id=${newerExperimentId}`,
          ),
      ),
    ).toBe(false);
  });
});

describe("stale expected evidence repair", () => {
  let persistedCase: ReturnType<typeof legacyCase>;
  let updateBodies: Record<string, unknown>[];
  let blockDatasetRefetch: boolean;
  let datasetRequestCount: number;
  let pendingDatasetRefetch: Deferred<Response> | null;

  beforeEach(() => {
    persistedCase = legacyCase();
    updateBodies = [];
    blockDatasetRefetch = false;
    datasetRequestCount = 0;
    pendingDatasetRefetch = null;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = input.toString();
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}`)) {
          datasetRequestCount += 1;
          if (
            blockDatasetRefetch &&
            datasetRequestCount > 1 &&
            pendingDatasetRefetch
          ) {
            return pendingDatasetRefetch.promise;
          }
          return responseJson({ ...dataset(), cases: [persistedCase] });
        }
        if (
          url.includes(`/api/v1/eval-lab/datasets/${datasetId}/experiments`)
        ) {
          return responseJson([]);
        }
        if (url.includes(`/api/v1/eval-lab/datasets/${datasetId}/trends`)) {
          return responseJson({ ...trendSummary(), points: [] });
        }
        if (url.endsWith("/api/v1/eval-lab/evidence/query")) {
          const request = requestBody(init);
          const requestedDocuments = stringArray(request.document_ids);
          const requestedChunks = stringArray(request.chunk_ids);
          const includeSearchResults = typeof request.query === "string";
          const lookup = evidenceLookup();
          return responseJson({
            documents:
              includeSearchResults || requestedDocuments.includes(documentId)
                ? lookup.documents
                : [],
            chunks:
              includeSearchResults || requestedChunks.includes(chunkId)
                ? lookup.chunks
                : [],
            unresolved_document_ids: requestedDocuments.filter(
              (id) => id === staleDocumentId,
            ),
            unresolved_chunk_ids: requestedChunks.filter(
              (id) => id === staleChunkId,
            ),
          });
        }
        if (
          url.endsWith(`/api/v1/eval-lab/cases/${caseId}`) &&
          init?.method === "PATCH"
        ) {
          const request = requestBody(init);
          updateBodies.push(request);
          if (
            stringArray(request.expected_document_ids).includes(
              staleDocumentId,
            ) ||
            stringArray(request.expected_chunk_ids).includes(staleChunkId)
          ) {
            return responseError(
              400,
              "bad request: Some selected evidence is unavailable. Remove or replace stale evidence before saving.",
            );
          }
          persistedCase = mergeCaseUpdate(persistedCase, request);
          return responseJson(persistedCase);
        }
        return responseJson([]);
      }),
    );
  });

  afterEach(() => vi.unstubAllGlobals());

  it("omits evidence fields for name, notes, and top_k-only edits", async () => {
    renderDatasetDetail();

    let caseCard = await openCaseEditor();
    fireEvent.change(within(caseCard).getByLabelText("Case name"), {
      target: { value: "Renamed legacy case" },
    });
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );
    await waitFor(() => expect(updateBodies).toHaveLength(1));
    expect(updateBodies[0]).not.toHaveProperty("expected_chunk_ids");
    expect(updateBodies[0]).not.toHaveProperty("expected_document_ids");

    caseCard = await openCaseEditor("Renamed legacy case");
    fireEvent.change(within(caseCard).getByLabelText("Notes"), {
      target: { value: "Updated without touching stale evidence." },
    });
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );
    await waitFor(() => expect(updateBodies).toHaveLength(2));
    expect(updateBodies[1]).not.toHaveProperty("expected_chunk_ids");
    expect(updateBodies[1]).not.toHaveProperty("expected_document_ids");

    caseCard = await openCaseEditor("Renamed legacy case");
    fireEvent.change(within(caseCard).getByLabelText("Results per question"), {
      target: { value: "9" },
    });
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );
    await waitFor(() => expect(updateBodies).toHaveLength(3));
    expect(updateBodies[2]).not.toHaveProperty("expected_chunk_ids");
    expect(updateBodies[2]).not.toHaveProperty("expected_document_ids");
  });

  it("clears existing notes and reopens with the persisted empty value", async () => {
    renderDatasetDetail();

    let caseCard = await openCaseEditor();
    fireEvent.change(within(caseCard).getByLabelText("Notes"), {
      target: { value: "" },
    });
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );

    await waitFor(() => expect(updateBodies).toHaveLength(1));
    expect(updateBodies[0]).toHaveProperty("notes", null);
    expect(persistedCase.notes).toBeNull();
    expect(screen.queryByText("Stored legacy note.")).not.toBeInTheDocument();

    caseCard = await openCaseEditor();
    expect(within(caseCard).getByLabelText("Notes")).toHaveValue("");
  });

  it("replaces stale evidence through complete normalized arrays", async () => {
    renderDatasetDetail();
    let caseCard = await openCaseEditor();

    await removeStaleEvidence(caseCard);
    fireEvent.click(
      (
        await within(caseCard).findAllByRole("button", {
          name: "Expect this exact chunk",
        })
      )[0],
    );
    fireEvent.click(
      (
        await within(caseCard).findAllByRole("button", {
          name: "Accept evidence from this document",
        })
      )[0],
    );
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );
    await waitFor(() => expect(updateBodies).toHaveLength(1));
    expect(updateBodies[0]).toMatchObject({
      expected_chunk_ids: [chunkId],
      expected_document_ids: [documentId],
    });

    caseCard = await openCaseEditor();
    fireEvent.change(within(caseCard).getByLabelText("Notes"), {
      target: { value: "Evidence snapshot was reset after saving." },
    });
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );
    await waitFor(() => expect(updateBodies).toHaveLength(2));
    expect(updateBodies[1]).not.toHaveProperty("expected_chunk_ids");
    expect(updateBodies[1]).not.toHaveProperty("expected_document_ids");
  });

  it("reopens from the synchronous cache while dataset refetch is pending", async () => {
    renderDatasetDetail();
    let caseCard = await openCaseEditor();
    blockDatasetRefetch = true;
    pendingDatasetRefetch = deferred<Response>();

    fireEvent.change(within(caseCard).getByLabelText("Case name"), {
      target: { value: "Cache-authoritative case" },
    });
    await removeStaleEvidence(caseCard);
    fireEvent.click(
      (
        await within(caseCard).findAllByRole("button", {
          name: "Expect this exact chunk",
        })
      )[0],
    );
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );

    await waitFor(() => expect(updateBodies).toHaveLength(1));
    expect(updateBodies[0]).toMatchObject({
      expected_chunk_ids: [chunkId],
      expected_document_ids: [],
    });

    caseCard = await openCaseEditor("Cache-authoritative case");
    expect(within(caseCard).getByLabelText("Case name")).toHaveValue(
      "Cache-authoritative case",
    );
    expect(
      await within(caseCard).findByRole("button", { name: "Remove chunk 2" }),
    ).toBeInTheDocument();
    expect(
      within(caseCard).queryByRole("button", { name: /Remove stale/ }),
    ).not.toBeInTheDocument();

    fireEvent.change(within(caseCard).getByLabelText("Results per question"), {
      target: { value: "11" },
    });
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );
    await waitFor(() => expect(updateBodies).toHaveLength(2));
    expect(updateBodies[1]).not.toHaveProperty("expected_chunk_ids");
    expect(updateBodies[1]).not.toHaveProperty("expected_document_ids");

    pendingDatasetRefetch.resolve(
      await responseJson({ ...dataset(), cases: [persistedCase] }),
    );
  });

  it("sends explicit empty arrays when all stale evidence is removed", async () => {
    renderDatasetDetail();
    const caseCard = await openCaseEditor();

    await removeStaleEvidence(caseCard);
    expect(
      within(caseCard).getByText(/Saving will clear this case's evidence/),
    ).toBeInTheDocument();
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );
    await waitFor(() => expect(updateBodies).toHaveLength(1));
    expect(updateBodies[0]).toMatchObject({
      expected_chunk_ids: [],
      expected_document_ids: [],
    });
  });

  it("shows strict validation errors without abandoning the stale selection", async () => {
    renderDatasetDetail();
    const caseCard = await openCaseEditor();

    fireEvent.click(
      await within(caseCard).findByRole("button", {
        name: /Remove stale document/,
      }),
    );
    fireEvent.change(within(caseCard).getByLabelText("Case name"), {
      target: { value: "Must not persist" },
    });
    fireEvent.click(
      within(caseCard).getByRole("button", { name: "Save changes" }),
    );

    expect(await within(caseCard).findByRole("alert")).toHaveTextContent(
      "Some selected evidence is unavailable. Remove or replace stale evidence before saving.",
    );
    expect(updateBodies[0]).toMatchObject({
      expected_chunk_ids: [staleChunkId],
      expected_document_ids: [],
    });
    expect(persistedCase.name).toBe("Legacy stale evidence");
    expect(
      within(caseCard).getByRole("button", { name: /Remove stale chunk/ }),
    ).toBeInTheDocument();
  });

  it("restores persisted fields and evidence after cancellation", async () => {
    renderDatasetDetail();
    let caseCard = await openCaseEditor();

    fireEvent.change(within(caseCard).getByLabelText("Case name"), {
      target: { value: "Abandoned name" },
    });
    fireEvent.change(within(caseCard).getByLabelText("Question"), {
      target: { value: "Abandoned query" },
    });
    fireEvent.change(within(caseCard).getByLabelText("Notes"), {
      target: { value: "Abandoned notes" },
    });
    fireEvent.change(within(caseCard).getByLabelText("Results per question"), {
      target: { value: "17" },
    });
    await removeStaleEvidence(caseCard);
    fireEvent.click(within(caseCard).getByRole("button", { name: "Cancel" }));

    caseCard = await openCaseEditor();
    expect(within(caseCard).getByLabelText("Case name")).toHaveValue(
      "Legacy stale evidence",
    );
    expect(within(caseCard).getByLabelText("Question")).toHaveValue(
      "Which legacy evidence is required?",
    );
    expect(within(caseCard).getByLabelText("Notes")).toHaveValue(
      "Stored legacy note.",
    );
    expect(within(caseCard).getByLabelText("Results per question")).toHaveValue(
      5,
    );
    expect(
      await within(caseCard).findByRole("button", {
        name: /Remove stale document/,
      }),
    ).toBeInTheDocument();
    expect(
      within(caseCard).getByRole("button", { name: /Remove stale chunk/ }),
    ).toBeInTheDocument();
    expect(updateBodies).toHaveLength(0);
  });

  function renderDatasetDetail() {
    renderRoute(
      `/app/evals/datasets/${datasetId}`,
      <Route
        path="/app/evals/datasets/:datasetId"
        element={<DatasetDetailPage />}
      />,
    );
  }

  async function openCaseEditor(name = persistedCase.name) {
    const editButton = await screen.findByRole("button", {
      name: `Edit ${name}`,
    });
    const caseCard = editButton.closest("article") as HTMLElement;
    fireEvent.click(editButton);
    return caseCard;
  }

  async function removeStaleEvidence(caseCard: HTMLElement) {
    fireEvent.click(
      await within(caseCard).findByRole("button", {
        name: /Remove stale document/,
      }),
    );
    fireEvent.click(
      await within(caseCard).findByRole("button", {
        name: /Remove stale chunk/,
      }),
    );
  }
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

function regression(): RetrievalEvalRegressionComparison {
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

function evidenceLookup() {
  return {
    documents: [
      {
        id: documentId,
        source_id: sourceId,
        source_name: "Platform docs",
        path: "platform-guide.md",
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
        source_name: "Platform docs",
        document_path: "platform-guide.md",
        ordinal: 0,
        text_preview: "GPU workers accelerate embedding indexing.",
        preview_truncated: false,
        token_count: 5,
        checksum: "1234567890abcdef",
        section_title: "Indexing",
        quality_flags: ["good_evidence_candidate"],
        is_duplicate: false,
        text_density: 0.9,
        evidence_score_hint: 0.8,
      },
      {
        id: retrievedChunkId,
        document_id: documentId,
        source_id: sourceId,
        source_name: "Platform docs",
        document_path: "platform-guide.md",
        ordinal: 1,
        text_preview: "A sibling chunk describes the indexing queue.",
        preview_truncated: false,
        token_count: 7,
        checksum: "fedcba0987654321",
        section_title: "Indexing queue",
        quality_flags: [],
        is_duplicate: false,
        text_density: 0.88,
        evidence_score_hint: 0.72,
      },
    ],
    unresolved_document_ids: [],
    unresolved_chunk_ids: [],
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
      modeResult("hybrid", 0.5, 0.4, 20, [caseEvaluation([retrievedChunkId])]),
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

function ciRun() {
  const experimentValue = experiment();
  return {
    id: ciRunId,
    workspace_id: "018f7a2a-6e2e-7000-a000-000000000320",
    dataset_id: datasetId,
    dataset_name: experimentValue.dataset_name,
    experiment_id: experimentId,
    status: "failed",
    gate_status: "failed",
    branch: "feature/ci-polish",
    commit_sha: "abc123def456",
    base_ref: "main",
    head_ref: "feature/ci-polish",
    config_label: "release-v2",
    regression: {
      baseline_run_id: "018f7a2a-6e2e-7000-a000-000000000321",
      recall_delta: -0.5,
      precision_delta: -0.6,
      mrr_delta: -0.5,
      latency_delta_ms: 5,
      newly_failed_case_count: 1,
      summary: "One case newly failed.",
    },
    eval_regression: regression(),
    report: {
      title: "Production corpus gate CI eval report",
      summary: "CI retrieval gate failed with one failure signal.",
      gate: experimentValue.gate,
      experiment: experimentValue,
      failed_cases: experimentValue.failures,
    },
    created_at: "2026-08-09T10:00:00Z",
  };
}

function experimentWithRetrievedChunks(retrievedChunkIds: string[]) {
  const value = experiment();
  value.mode_results[0].case_results = [caseEvaluation(retrievedChunkIds)];
  return value;
}

function caseEvaluation(retrievedChunkIds: string[]) {
  return {
    case_id: caseId,
    query: "Which evidence explains GPU indexing workers?",
    top_k: 5,
    recall_at_k: retrievedChunkIds.includes(chunkId) ? 1 : 0,
    precision_at_k: 0,
    mrr: 0,
    top_hit_rank: retrievedChunkIds.length > 0 ? 1 : null,
    citation_coverage: 0,
    weak_evidence_count: 0,
    missing_embedding_failures: 0,
    passed: retrievedChunkIds.includes(chunkId),
    expected_chunk_ids: [chunkId],
    expected_document_ids: [documentId],
    retrieved_chunk_ids: retrievedChunkIds,
    latency_ms: 20,
    failures:
      retrievedChunkIds.length > 0
        ? [
            {
              case_id: caseId,
              query: "Which evidence explains GPU indexing workers?",
              retrieval_mode: "hybrid",
              label: "correct_document_wrong_chunk",
              severity: "warning",
              message:
                "The correct document ranked, but the exact chunk did not.",
              top_hit_rank: 1,
            },
          ]
        : [],
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
  caseResults: ReturnType<typeof caseEvaluation>[] = [],
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
    case_results: caseResults,
  };
}

function gate(status: "passed" | "failed") {
  return {
    status,
    average_recall_at_k: status === "passed" ? 1 : 0.5,
    weak_evidence_rate: status === "passed" ? 0 : 1,
    critical_failure_count: status === "failed" ? 1 : 0,
    recall_threshold: 0.8,
    weak_evidence_limit: 0.2,
    reasons:
      status === "failed"
        ? ["Average recall is below 80%."]
        : ["All gate rules passed."],
  };
}

function noBaselineRegression(): RetrievalEvalRegressionComparison {
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

function legacyCase(): RetrievalEvalCase {
  return {
    ...dataset().cases[0],
    name: "Legacy stale evidence",
    query: "Which legacy evidence is required?",
    expected_chunk_ids: [staleChunkId],
    expected_document_ids: [staleDocumentId],
    notes: "Stored legacy note.",
  };
}

function requestBody(init?: RequestInit): Record<string, unknown> {
  return typeof init?.body === "string"
    ? (JSON.parse(init.body) as Record<string, unknown>)
    : {};
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

function mergeCaseUpdate(
  current: RetrievalEvalCase,
  request: Record<string, unknown>,
): RetrievalEvalCase {
  return {
    ...current,
    name: typeof request.name === "string" ? request.name : current.name,
    query: typeof request.query === "string" ? request.query : current.query,
    top_k: typeof request.top_k === "number" ? request.top_k : current.top_k,
    expected_chunk_ids: Object.hasOwn(request, "expected_chunk_ids")
      ? stringArray(request.expected_chunk_ids)
      : current.expected_chunk_ids,
    expected_document_ids: Object.hasOwn(request, "expected_document_ids")
      ? stringArray(request.expected_document_ids)
      : current.expected_document_ids,
    notes: Object.hasOwn(request, "notes")
      ? typeof request.notes === "string"
        ? request.notes
        : null
      : current.notes,
  };
}

function responseJson(json: unknown) {
  return Promise.resolve({ status: 200, json: async () => json } as Response);
}

function responseError(status: number, message: string) {
  return Promise.resolve({
    status,
    text: async () => JSON.stringify({ error: { message } }),
  } as Response);
}

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}
