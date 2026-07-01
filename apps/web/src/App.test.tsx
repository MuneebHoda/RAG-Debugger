import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";
import { queryClient } from "./app/queryClient";
import {
  clearAuthSession,
  createAuthSession,
  DEMO_CREDENTIALS,
} from "./features/auth/authSession";

describe("App", () => {
  afterEach(() => {
    queryClient.clear();
    clearAuthSession();
    vi.unstubAllGlobals();
  });

  it("renders the CorpusLab public site", async () => {
    render(
      <MemoryRouter>
        <App />
      </MemoryRouter>,
    );

    expect(
      await screen.findByRole("heading", {
        name: /see why your rag answer failed/i,
      }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/CorpusLab/i).length).toBeGreaterThan(0);
    expect(
      screen.getAllByRole("link", { name: /features/i }).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByRole("link", { name: /pricing/i }).length,
    ).toBeGreaterThan(0);
  });

  it("renders the workbench under the app route", async () => {
    createAuthSession(DEMO_CREDENTIALS.email, "Demo User");
    stubWorkbenchFetch();

    render(
      <MemoryRouter initialEntries={["/app"]}>
        <App />
      </MemoryRouter>,
    );

    expect(
      await screen.findByRole("heading", { name: /^home$/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /corpus/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /^runs$/i })).toHaveAttribute(
      "href",
      "/app/traces",
    );
  });

  it("redirects unauthenticated workbench visits to login", () => {
    render(
      <MemoryRouter initialEntries={["/app"]}>
        <App />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", { name: /sign in/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(DEMO_CREDENTIALS.email)).toBeInTheDocument();
  });
});

function stubWorkbenchFetch() {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      const url = input.toString();
      if (url.endsWith("/healthz")) {
        return responseJson({ status: "ok" });
      }
      if (url.endsWith("/api/v1/auth/me")) {
        return responseJson({
          user: {
            user: {
              id: "018f7a2a-6e2e-7000-a000-000000000401",
              email: DEMO_CREDENTIALS.email,
              name: "Demo User",
              created_at: "2026-06-25T00:00:00Z",
            },
            organization: {
              id: "018f7a2a-6e2e-7000-a000-000000000402",
              name: "CorpusLab Demo Organization",
              created_at: "2026-06-25T00:00:00Z",
            },
            workspace: {
              id: "018f7a2a-6e2e-7000-a000-000000000403",
              organization_id: "018f7a2a-6e2e-7000-a000-000000000402",
              name: "Corpus Demo Workspace",
              created_at: "2026-06-25T00:00:00Z",
            },
            role: "owner",
          },
        });
      }
      if (url.endsWith("/api/v1/config")) {
        return responseJson({
          product: {
            name: "CorpusLab",
            workspace_name: "Corpus Demo Workspace",
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
          embedding: {
            model: {
              provider: "local",
              model_name: "local-hash-v1",
              dimension: 384,
            },
            provider_kind: "local_hash",
          },
          ui: {
            api_base_url: "http://127.0.0.1:8080",
            show_local_badges: true,
          },
        });
      }
      if (url.endsWith("/api/v1/overview")) {
        return responseJson({
          generated_at: "2026-06-25T00:00:00Z",
          health: {
            score: 0,
            status: "needs_documents",
            summary: "Ingest documents to begin corpus operations.",
            primary_action: {
              id: "ingest_documents",
              label: "Ingest documents",
              detail: "Add the first corpus source.",
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
          embedding_status: {
            model: {
              provider: "local",
              model_name: "local-hash-v1",
              dimension: 384,
            },
            total_chunks: 0,
            indexed_chunks: 0,
            missing_chunks: 0,
            stale_chunks: 0,
            last_indexed_at: null,
          },
          latest_eval_run: null,
        });
      }
      return responseJson({});
    }),
  );
}

function responseJson(json: unknown) {
  return {
    status: 200,
    json: async () => json,
  } as Response;
}
