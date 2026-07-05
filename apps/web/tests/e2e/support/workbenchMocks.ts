import type { Page, Route } from "@playwright/test";

import type { ProductConfig } from "../../../src/lib/api/config";
import type { DemoStatus } from "../../../src/lib/api/demo";
import type { EmbeddingStatus } from "../../../src/lib/api/embeddings";
import type {
  RetrievalEvalDatasetSummary,
  RetrievalEvalExperiment,
} from "../../../src/lib/api/evalLab";
import type { OverviewResponse } from "../../../src/lib/api/overview";
import type { DebugReport } from "../../../src/lib/api/reports";
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
  await fulfillJson(page, "**/api/v1/eval-lab/experiments", state.experiments);
  await fulfillJson(page, "**/api/v1/eval-lab/ci/runs", []);
  await fulfillJson(page, "**/api/v1/reports", state.reports);
  await fulfillJson(page, "**/api/v1/api-keys", state.apiKeys);

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

async function fulfillJson(page: Page, url: string, json: unknown) {
  await page.route(url, (route: Route) =>
    route.fulfill({ contentType: "application/json", json }),
  );
}
