import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";
import {
  clearAuthSession,
  createAuthSession,
  DEMO_CREDENTIALS,
} from "./features/auth/authSession";

describe("App", () => {
  afterEach(() => {
    clearAuthSession();
    vi.unstubAllGlobals();
  });

  it("renders the CorpusLab public site", () => {
    render(
      <MemoryRouter>
        <App />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", {
        name: /turn every corpus into trusted retrieval/i,
      }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/CorpusLab/i).length).toBeGreaterThan(0);
    expect(screen.getByRole("link", { name: /features/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /pricing/i })).toBeInTheDocument();
  });

  it("renders the workbench under the app route", () => {
    createAuthSession(DEMO_CREDENTIALS.email);
    stubWorkbenchFetch();

    render(
      <MemoryRouter initialEntries={["/app"]}>
        <App />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", { name: /mission control/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /sources/i })).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /ingest documents/i }),
    ).toHaveAttribute("href", "/app/sources");
    expect(screen.getByRole("link", { name: /open traces/i })).toHaveAttribute(
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
