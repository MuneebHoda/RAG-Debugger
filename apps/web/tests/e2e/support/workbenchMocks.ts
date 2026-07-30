import type { Page, Route } from "@playwright/test";

import type { ProductConfig } from "../../../src/lib/api/config";
import type { DemoStatus } from "../../../src/lib/api/demo";
import type { EmbeddingStatus } from "../../../src/lib/api/embeddings";
import type {
  EvalLabEvidenceChunk,
  EvalLabEvidenceDocument,
  RetrievalEvalDatasetSummary,
  RetrievalEvalExperiment,
  RetrievalEvalExperimentSummary,
  RetrievalEvalRegressionComparison,
} from "../../../src/lib/api/evalLab";
import type { OverviewResponse } from "../../../src/lib/api/overview";
import type { DebugReport } from "../../../src/lib/api/reports";
import type { RetrievalQueryResponse } from "../../../src/lib/api/retrieval";
import type { ChunkPreview, SourceSummary } from "../../../src/lib/api/sources";
import type { Trace, TraceSummary } from "../../../src/lib/api/traces";
import type { ApiKey } from "../../../src/lib/api/apiKeys";
import { authResponse, authSession } from "./auth";

export interface WorkbenchMockState {
  sources: SourceSummary[];
  documentChunks: Record<string, ChunkPreview[]>;
  traces: TraceSummary[];
  traceDetails: Record<string, Trace>;
  datasets: RetrievalEvalDatasetSummary[];
  experiments: RetrievalEvalExperiment[];
  experimentDetails: Record<string, RetrievalEvalExperiment>;
  reports: DebugReport[];
  reportDetails: Record<string, DebugReport>;
  apiKeys: ApiKey[];
  retrievalResponse: RetrievalQueryResponse | null;
}

const productConfig = {
  product: {
    name: "CorpusLab",
    workspace_name: authSession.workspaceName,
    deployment_mode: "local",
  },
  ingestion: {
    max_files_per_request: 10,
    max_file_bytes: 20_971_520,
    max_request_bytes: 52_428_800,
    preview_chunk_limit: 8,
    supported_extensions: ["txt", "md", "pdf"],
  },
  chunking: {
    target_tokens: 512,
    overlap_tokens: 64,
    strategy: "structured",
  },
  retrieval: {
    default_top_k: 5,
    max_top_k: 25,
    default_mode: "hybrid",
    min_evidence_score: 0.35,
    min_semantic_similarity: 0.25,
    answer_citation_limit: 3,
    answerability: {
      min_body_term_coverage: 0.5,
      min_body_term_matches: 2,
    },
    weights: {},
  },
  debugger: { low_score_margin_ratio: 0.1 },
  embedding: {
    model: { provider: "local", model_name: "local-hash-v1", dimension: 384 },
    provider_kind: "local_hash",
  },
  ui: { api_base_url: "http://127.0.0.1:18080", show_local_badges: true },
} satisfies ProductConfig;

const embeddingStatus = {
  model: productConfig.embedding.model,
  total_chunks: 0,
  indexed_chunks: 0,
  missing_chunks: 0,
  stale_chunks: 0,
  last_indexed_at: null,
} satisfies EmbeddingStatus;

const overview = {
  generated_at: "2026-07-04T08:00:00Z",
  health: {
    score: 0,
    status: "needs_documents",
    summary: "Add documents to begin testing retrieval.",
    primary_action: {
      id: "ingest",
      label: "Add documents",
      detail: "Build the corpus.",
      route: "/app/sources",
      priority: "primary",
    },
  },
  metrics: [],
  pipeline: [],
  issues: [],
  actions: [],
  recent_activity: [],
  document_mix: [],
  embedding_status: embeddingStatus,
  latest_eval_run: null,
  latest_eval_experiment: null,
} satisfies OverviewResponse;

const demoStatus = {
  version: "corpuslab-guided-demo-v1",
  project_id: null,
  source_id: null,
  progress: {
    sample_corpus_loaded: false,
    chunks_created: false,
    embeddings_indexed: false,
    document_count: 0,
    chunk_count: 0,
    indexed_chunk_count: 0,
    retrieval_run_id: null,
    trace_id: null,
    report_id: null,
  },
  suggested_queries: [],
} satisfies DemoStatus;

export async function installWorkbenchMocks(
  page: Page,
  overrides: Partial<WorkbenchMockState> = {},
) {
  const state: WorkbenchMockState = {
    sources: [],
    documentChunks: {},
    traces: [],
    traceDetails: {},
    datasets: [],
    experiments: [],
    experimentDetails: {},
    reports: [],
    reportDetails: {},
    apiKeys: [],
    retrievalResponse: null,
    ...overrides,
  };

  await page.addInitScript((session) => {
    window.localStorage.setItem(
      "corpuslab.auth.session",
      JSON.stringify(session),
    );
  }, authSession);

  await fulfillJson(page, "**/healthz", { status: "ok" });
  await fulfillJson(page, "**/api/v1/auth/me", authResponse);
  await fulfillJson(page, "**/api/v1/config", productConfig);
  await fulfillJson(page, "**/api/v1/overview", overview);
  await fulfillJson(page, "**/api/v1/demo", demoStatus);
  await fulfillJson(page, "**/api/v1/sources", state.sources);
  await fulfillJson(page, "**/api/v1/embeddings/status", embeddingStatus);
  await fulfillJson(page, "**/api/v1/retrieval/evals", []);
  await fulfillJson(page, "**/api/v1/traces", state.traces);
  await fulfillJson(page, "**/api/v1/eval-lab/datasets", state.datasets);
  await page.route("**/api/v1/eval-lab/evidence/query", (route) =>
    route.fulfill({
      contentType: "application/json",
      json: evidenceLookup(state),
    }),
  );
  await fulfillJson(
    page,
    "**/api/v1/eval-lab/datasets/*/experiments",
    allExperiments(state).map(summarizeExperiment),
  );
  await page.route("**/api/v1/eval-lab/datasets/*", (route) => {
    const datasetId = route.request().url().split("/").at(-1);
    const dataset = state.datasets.find((item) => item.id === datasetId);
    return route.fulfill({
      contentType: "application/json",
      json: {
        id: dataset?.id ?? datasetId,
        name: dataset?.name ?? "Mock dataset",
        description: dataset?.description ?? null,
        cases: [],
        created_at: dataset?.updated_at ?? "2026-07-04T08:00:00Z",
        updated_at: dataset?.updated_at ?? "2026-07-04T08:00:00Z",
      },
    });
  });
  await fulfillJson(page, "**/api/v1/eval-lab/experiments", state.experiments);
  await fulfillRegression(page, state);
  await fulfillJson(page, "**/api/v1/eval-lab/ci/runs", []);
  await fulfillJson(page, "**/api/v1/reports", state.reports);
  await fulfillJson(page, "**/api/v1/api-keys", state.apiKeys);

  if (state.retrievalResponse) {
    await fulfillJson(
      page,
      "**/api/v1/retrieval/query",
      state.retrievalResponse,
    );
  }

  for (const [id, trace] of Object.entries(state.traceDetails)) {
    await fulfillJson(page, `**/api/v1/traces/${id}`, trace);
  }
  for (const [id, experiment] of Object.entries(state.experimentDetails)) {
    await fulfillJson(page, `**/api/v1/eval-lab/experiments/${id}`, experiment);
  }
  for (const [id, report] of Object.entries(state.reportDetails)) {
    await fulfillJson(page, `**/api/v1/reports/${id}`, report);
  }
  for (const [id, chunks] of Object.entries(state.documentChunks)) {
    await fulfillJson(page, `**/api/v1/documents/${id}/chunks`, chunks);
  }

  return state;
}

function evidenceLookup(state: WorkbenchMockState) {
  const documents: EvalLabEvidenceDocument[] = [];
  const chunks: EvalLabEvidenceChunk[] = [];
  for (const source of state.sources) {
    for (const summary of source.documents) {
      documents.push({
        id: summary.document.id,
        source_id: source.source.id,
        source_name: source.source.name,
        path: summary.document.path,
        profile: summary.document.profile,
        extraction_quality: summary.document.extraction_quality,
        warnings: summary.document.warnings,
        chunk_count: summary.chunk_count,
      });
      for (const chunk of state.documentChunks[summary.document.id] ?? []) {
        chunks.push({
          id: chunk.id,
          document_id: chunk.document_id,
          source_id: source.source.id,
          source_name: source.source.name,
          document_path: summary.document.path,
          ordinal: chunk.ordinal,
          text_preview: chunk.text,
          preview_truncated: false,
          token_count: chunk.token_count,
          checksum: chunk.checksum,
          section_title: chunk.section_title,
          quality_flags: chunk.quality_flags,
          is_duplicate: chunk.is_duplicate,
          text_density: chunk.text_density,
          evidence_score_hint: chunk.evidence_score_hint,
        });
      }
    }
  }

  return {
    documents,
    chunks,
    unresolved_document_ids: [],
    unresolved_chunk_ids: [],
  };
}

async function fulfillJson(page: Page, url: string, json: unknown) {
  await page.route(url, (route: Route) =>
    route.fulfill({ contentType: "application/json", json }),
  );
}

async function fulfillRegression(page: Page, state: WorkbenchMockState) {
  await page.route("**/api/v1/eval-lab/experiments/*/regression*", (route) => {
    const url = new URL(route.request().url());
    const experimentId = url.pathname.split("/").at(-2);
    const current = experimentId
      ? allExperiments(state).find(
          (experiment) => experiment.id === experimentId,
        )
      : null;
    const baselineId = url.searchParams.get("baseline_id");
    const baseline =
      baselineId && current
        ? allExperiments(state).find(
            (experiment) => experiment.id === baselineId,
          )
        : current
          ? previousExperiment(current, allExperiments(state))
          : null;

    return route.fulfill({
      contentType: "application/json",
      json: regressionComparison(current, baseline ?? null),
    });
  });
}

function allExperiments(state: WorkbenchMockState) {
  const experiments = new Map<string, RetrievalEvalExperiment>();
  for (const experiment of state.experiments) {
    experiments.set(experiment.id, experiment);
  }
  for (const experiment of Object.values(state.experimentDetails)) {
    experiments.set(experiment.id, experiment);
  }
  return [...experiments.values()];
}

function summarizeExperiment(
  experiment: RetrievalEvalExperiment,
): RetrievalEvalExperimentSummary {
  const bestResult =
    experiment.mode_results.find(
      (result) => result.retrieval_mode === experiment.comparison.best_mode,
    ) ?? experiment.mode_results[0];
  return {
    id: experiment.id,
    dataset_id: experiment.dataset_id,
    dataset_name: experiment.dataset_name,
    name: experiment.name,
    modes: experiment.modes,
    top_k: experiment.top_k,
    best_mode: experiment.comparison.best_mode,
    gate_status: experiment.gate.status,
    average_recall_at_k: bestResult?.average_recall_at_k ?? 0,
    average_precision_at_k: bestResult?.average_precision_at_k ?? 0,
    mean_reciprocal_rank: bestResult?.mean_reciprocal_rank ?? 0,
    citation_coverage: bestResult?.citation_coverage ?? 0,
    weak_evidence_case_rate: bestResult
      ? bestResult.weak_evidence_count / Math.max(bestResult.case_count, 1)
      : 0,
    missing_embedding_failures: bestResult?.missing_embedding_failures ?? 0,
    latency_p50_ms: bestResult?.latency_p50_ms ?? 0,
    latency_p95_ms: bestResult?.latency_p95_ms ?? 0,
    failure_count: experiment.failures.length,
    created_at: experiment.created_at,
  };
}

function previousExperiment(
  current: RetrievalEvalExperiment,
  experiments: RetrievalEvalExperiment[],
) {
  const currentTime = Date.parse(current.created_at);
  return experiments
    .filter(
      (experiment) =>
        experiment.dataset_id === current.dataset_id &&
        experiment.id !== current.id &&
        Date.parse(experiment.created_at) < currentTime,
    )
    .sort(
      (left, right) =>
        Date.parse(right.created_at) - Date.parse(left.created_at),
    )
    .at(0);
}

function regressionComparison(
  current: RetrievalEvalExperiment | null | undefined,
  baseline: RetrievalEvalExperiment | null,
): RetrievalEvalRegressionComparison {
  return {
    current_experiment_id: current?.id ?? "missing",
    baseline_experiment_id: baseline?.id ?? null,
    classification: baseline ? "regressed" : "unchanged",
    current_gate_status: current?.gate.status ?? "failed",
    baseline_gate_status: baseline?.gate.status ?? null,
    metric_deltas: [
      {
        metric: "recall_at_k",
        current: current?.gate.average_recall_at_k ?? 0,
        baseline: baseline?.gate.average_recall_at_k ?? null,
        delta: baseline
          ? (current?.gate.average_recall_at_k ?? 0) -
            baseline.gate.average_recall_at_k
          : 0,
        classification: baseline ? "regressed" : "unchanged",
      },
    ],
    newly_failed_cases: [],
    recovered_cases: [],
    changed_top_evidence_cases: [],
    changed_failure_label_cases: [],
    summary: baseline
      ? "Current experiment is compared with the selected baseline."
      : "No comparable baseline exists.",
  };
}
