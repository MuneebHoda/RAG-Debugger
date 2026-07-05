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

import {
  clearAuthSession,
  createAuthSession,
} from "../features/auth/authSession";
import { logout } from "../lib/api/auth";
import { getProductConfig } from "../lib/api/config";
import { getHealth } from "../lib/api/health";
import { getOverview } from "../lib/api/overview";
import { WorkbenchLayout } from "./WorkbenchLayout";

vi.mock("../lib/api/auth", () => ({ logout: vi.fn() }));
vi.mock("../lib/api/config", () => ({ getProductConfig: vi.fn() }));
vi.mock("../lib/api/health", () => ({ getHealth: vi.fn() }));
vi.mock("../lib/api/overview", () => ({ getOverview: vi.fn() }));

describe("WorkbenchLayout", () => {
  beforeEach(() => {
    createAuthSession("demo@corpuslab.ai", "Demo User");
    vi.mocked(getProductConfig).mockResolvedValue(productConfig());
    vi.mocked(getHealth).mockResolvedValue({ status: "ok" });
    vi.mocked(getOverview).mockResolvedValue(overview());
    vi.mocked(logout).mockResolvedValue({ ok: true });
  });

  afterEach(() => {
    clearAuthSession();
    vi.clearAllMocks();
  });

  it("groups the product workflow and identifies the active route", async () => {
    renderLayout("/app/traces");

    const navigation = screen.getByRole("complementary", {
      name: "Workspace navigation",
    });
    expect(within(navigation).getByText("Setup")).toBeInTheDocument();
    expect(within(navigation).getByText("Debug")).toBeInTheDocument();
    expect(within(navigation).getByText("Quality")).toBeInTheDocument();
    expect(within(navigation).getByText("Share")).toBeInTheDocument();
    expect(within(navigation).getByText("Admin")).toBeInTheDocument();
    expect(
      within(navigation).getByRole("link", { name: "Trace Debugger" }),
    ).toHaveAttribute("aria-current", "page");
    expect(
      await screen.findByRole("navigation", { name: "Breadcrumb" }),
    ).toHaveTextContent("HomeTrace Debugger");
  });

  it("keeps CI Runs distinct from the Eval Lab active state", () => {
    renderLayout("/app/evals?view=ci-runs");

    const navigation = screen.getByRole("complementary", {
      name: "Workspace navigation",
    });
    expect(
      within(navigation).getByRole("link", { name: "CI Runs" }),
    ).toHaveAttribute("aria-current", "page");
    expect(
      within(navigation).getByRole("link", { name: "Eval Lab" }),
    ).not.toHaveAttribute("aria-current");
  });

  it("renders linked parent breadcrumbs for detail routes", () => {
    renderLayout("/app/traces/trace-1");

    const breadcrumbs = screen.getByRole("navigation", {
      name: "Breadcrumb",
    });
    expect(
      within(breadcrumbs).getByRole("link", { name: "Home" }),
    ).toHaveAttribute("href", "/app");
    expect(
      within(breadcrumbs).getByRole("link", { name: "Trace Debugger" }),
    ).toHaveAttribute("href", "/app/traces");
    expect(within(breadcrumbs).getByText("Run detail")).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("moves focus into mobile navigation and restores it with Escape", async () => {
    renderLayout("/app/sources");

    const menuButton = screen.getByRole("button", {
      name: "Open navigation",
    });
    fireEvent.click(menuButton);
    expect(menuButton).toHaveAttribute("aria-expanded", "true");
    await waitFor(() =>
      expect(screen.getByRole("link", { name: "Corpus" })).toHaveFocus(),
    );
    expect(document.body.style.overflow).toBe("hidden");

    fireEvent.keyDown(window, { key: "Escape" });
    expect(menuButton).toHaveAttribute("aria-expanded", "false");
    expect(menuButton).toHaveFocus();
    await waitFor(() => expect(document.body.style.overflow).toBe(""));
  });
});

function renderLayout(initialEntry: string) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <Routes>
          <Route path="/app" element={<WorkbenchLayout />}>
            <Route path="sources" element={<h1>Corpus route</h1>} />
            <Route path="traces" element={<h1>Trace Debugger route</h1>} />
            <Route path="traces/:traceId" element={<h1>Run detail</h1>} />
            <Route path="evals" element={<h1>Eval Lab route</h1>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function productConfig() {
  return {
    product: {
      name: "CorpusLab",
      workspace_name: "Corpus Demo Workspace",
      deployment_mode: "local" as const,
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
      strategy: "structured" as const,
    },
    retrieval: {
      default_top_k: 5,
      max_top_k: 25,
      default_mode: "hybrid" as const,
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
      provider_kind: "local_hash" as const,
    },
    ui: { api_base_url: "http://127.0.0.1:8080", show_local_badges: true },
  };
}

function overview() {
  return {
    generated_at: "2026-07-04T08:00:00Z",
    health: {
      score: 0,
      status: "needs_documents" as const,
      summary: "Add documents.",
      primary_action: null,
    },
    metrics: [],
    pipeline: [],
    issues: [],
    actions: [],
    recent_activity: [],
    document_mix: [],
    embedding_status: {
      model: { provider: "local", model_name: "local-hash-v1", dimension: 384 },
      total_chunks: 0,
      indexed_chunks: 0,
      missing_chunks: 0,
      stale_chunks: 0,
      last_indexed_at: null,
    },
    latest_eval_run: null,
    latest_eval_experiment: null,
  };
}
