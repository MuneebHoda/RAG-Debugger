import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { OverviewPage } from "./OverviewPage";

describe("OverviewPage", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => responseJson(baseOverview())),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders guided setup, core metrics, risks, and activity", async () => {
    renderOverview();

    expect(
      await screen.findByRole("heading", { name: /^home$/i }),
    ).toBeInTheDocument();
    expect(await screen.findByText(/workflow ready/i)).toBeInTheDocument();
    expect(screen.getByText("Documents")).toBeInTheDocument();
    expect(screen.getByText("What needs attention")).toBeInTheDocument();
    expect(screen.getByText("Recent system events")).toBeInTheDocument();

    fireEvent.click(screen.getByText("System details"));
    expect(screen.getByText("Profiles")).toBeInTheDocument();
  });

  it("guides an empty workspace toward Sources", async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      responseJson({
        ...baseOverview(),
        health: {
          score: 0,
          status: "needs_documents",
          summary: "Ingest documents to begin.",
          primary_action: {
            id: "ingest_documents",
            label: "Ingest documents",
            detail: "Add the first corpus source.",
            route: "/app/sources",
            priority: "primary",
          },
        },
        metrics: metricSet({ documents: "0", chunks: "0" }),
        pipeline: baseOverview().pipeline.map((pipelineStep) => ({
          ...pipelineStep,
          count: 0,
          status: "pending",
        })),
        issues: [
          {
            id: "no_documents",
            severity: "critical",
            title: "No documents ingested",
            detail: "CorpusLab needs documents before retrieval can run.",
            route: "/app/sources",
            action_label: "Ingest documents",
          },
        ],
        document_mix: [],
        embedding_status: {
          ...baseOverview().embedding_status,
          total_chunks: 0,
          indexed_chunks: 0,
        },
      }),
    );

    renderOverview();

    expect(
      await screen.findByText(/get to a trusted retrieval result/i),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /open corpus/i })).toHaveAttribute(
      "href",
      "/app/sources",
    );
  });

  it("recommends indexing when embeddings are missing", async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      responseJson({
        ...baseOverview(),
        health: {
          score: 70,
          status: "needs_indexing",
          summary: "Embeddings need attention.",
          primary_action: {
            id: "index_embeddings",
            label: "Index embeddings",
            detail: "Refresh local embeddings.",
            route: "/app/retrieval",
            priority: "primary",
          },
        },
        issues: [
          {
            id: "missing_embeddings",
            severity: "warning",
            title: "Missing embeddings",
            detail: "2 chunks need local embeddings.",
            route: "/app/retrieval",
            action_label: "Index embeddings",
          },
        ],
        embedding_status: {
          ...baseOverview().embedding_status,
          indexed_chunks: 2,
          missing_chunks: 2,
        },
      }),
    );

    renderOverview();

    expect(await screen.findByText(/indexing needed/i)).toBeInTheDocument();
    expect(screen.getByText("Missing embeddings")).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /index evidence/i }),
    ).toHaveAttribute("href", "/app/retrieval");
  });

  it("surfaces weak trace risk and links to the Trace Debugger", async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      responseJson({
        ...baseOverview(),
        issues: [
          {
            id: "weak_traces",
            severity: "warning",
            title: "Weak trace evidence",
            detail: "1 trace contains weak evidence or failure labels.",
            route: "/app/traces",
            action_label: "Debug traces",
          },
        ],
        actions: [
          {
            id: "review_weak_traces",
            label: "Review weak traces",
            detail: "Inspect failure labels and rerun retrieval modes.",
            route: "/app/traces",
            priority: "primary",
          },
        ],
      }),
    );

    renderOverview();

    expect(await screen.findByText("Weak trace evidence")).toBeInTheDocument();
    await waitFor(() => {
      expect(
        screen.getAllByRole("link", { name: /debug traces/i })[0],
      ).toHaveAttribute("href", "/app/traces");
    });
  });
});

function renderOverview() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <OverviewPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function baseOverview() {
  return {
    generated_at: "2026-06-25T00:00:00Z",
    health: {
      score: 92,
      status: "ready",
      summary: "Corpus operations are ready for retrieval review.",
      primary_action: {
        id: "run_retrieval",
        label: "Run retrieval",
        detail: "Ask a question and inspect cited evidence.",
        route: "/app/retrieval",
        priority: "primary",
      },
    },
    metrics: metricSet(),
    pipeline: [
      step(
        "ingest",
        "Ingest",
        "complete",
        2,
        "documents available",
        "/app/sources",
      ),
      step("chunk", "Chunk", "complete", 4, "retrieval units", "/app/sources"),
      step("embed", "Embed", "complete", 4, "chunks indexed", "/app/retrieval"),
      step(
        "retrieve",
        "Retrieve",
        "complete",
        1,
        "saved evidence runs",
        "/app/retrieval",
      ),
      step(
        "trace",
        "Trace",
        "complete",
        1,
        "debugger timelines",
        "/app/traces",
      ),
      step("eval", "Eval", "complete", 1, "coverage cases", "/app/evals"),
      step(
        "report",
        "Report",
        "complete",
        1,
        "trace-backed reports",
        "/app/reports",
      ),
    ],
    issues: [],
    actions: [
      {
        id: "run_retrieval",
        label: "Run retrieval",
        detail: "Ask a question and inspect cited evidence.",
        route: "/app/retrieval",
        priority: "primary",
      },
    ],
    recent_activity: [
      {
        id: "trace:1",
        kind: "trace",
        label: "gpu indexing workers",
        detail: "strong evidence · 12 ms",
        route: "/app/traces",
        created_at: "2026-06-25T00:00:00Z",
      },
    ],
    document_mix: [
      {
        profile: "technical_docs",
        count: 2,
        percentage: 1,
      },
    ],
    embedding_status: {
      model: {
        provider: "local",
        model_name: "local-hash-v1",
        dimension: 384,
      },
      total_chunks: 4,
      indexed_chunks: 4,
      missing_chunks: 0,
      stale_chunks: 0,
      last_indexed_at: "2026-06-25T00:00:00Z",
    },
    latest_eval_run: {
      id: "018f7a2a-6e2e-7000-a000-000000000001",
      retrieval_mode: "hybrid",
      case_count: 1,
      passed_count: 1,
      pass_rate: 1,
      average_recall_at_k: 1,
      average_precision_at_k: 1,
      created_at: "2026-06-25T00:00:00Z",
    },
  };
}

function metricSet(overrides: Record<string, string> = {}) {
  return [
    metric(
      "sources",
      "Sources",
      overrides.sources ?? "1",
      "connected corpora",
      "neutral",
    ),
    metric(
      "documents",
      "Documents",
      overrides.documents ?? "2",
      "indexed files",
      "neutral",
    ),
    metric(
      "chunks",
      "Chunks",
      overrides.chunks ?? "4",
      "retrieval units",
      "neutral",
    ),
    metric(
      "embeddings",
      "Embeddings",
      overrides.embeddings ?? "4/4",
      "0 missing · 0 stale",
      "good",
    ),
    metric(
      "traces",
      "Traces",
      overrides.traces ?? "1",
      "0 weak · 0 failed",
      "good",
    ),
    metric(
      "evals",
      "Eval coverage",
      overrides.evals ?? "1",
      "latest pass rate 100%",
      "good",
    ),
    metric(
      "warnings",
      "Warnings",
      overrides.warnings ?? "0",
      "extraction and corpus quality",
      "good",
    ),
  ];
}

function metric(
  id: string,
  label: string,
  value: string,
  detail: string,
  tone: string,
) {
  return { id, label, value, detail, tone };
}

function step(
  id: string,
  label: string,
  status: string,
  count: number,
  detail: string,
  route: string,
) {
  return {
    id,
    label,
    status,
    count,
    detail,
    route,
    action_label: `Open ${label}`,
  };
}

function responseJson(json: unknown) {
  return {
    status: 200,
    json: async () => json,
  } as Response;
}
