import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createApiKey, listApiKeys } from "../../../lib/api/apiKeys";
import { getCurrentUser } from "../../../lib/api/auth";
import { getProductConfig } from "../../../lib/api/config";
import { SettingsPage } from "./SettingsPage";

vi.mock("../../../lib/api/apiKeys", () => ({
  createApiKey: vi.fn(),
  listApiKeys: vi.fn(),
  revokeApiKey: vi.fn(),
}));
vi.mock("../../../lib/api/auth", () => ({ getCurrentUser: vi.fn() }));
vi.mock("../../../lib/api/config", () => ({ getProductConfig: vi.fn() }));

describe("SettingsPage", () => {
  beforeEach(() => {
    vi.mocked(getProductConfig).mockResolvedValue(productConfig());
    vi.mocked(getCurrentUser).mockResolvedValue(currentUser());
    vi.mocked(listApiKeys).mockResolvedValue([]);
  });

  afterEach(() => vi.clearAllMocks());

  it("exposes keyboard-reachable settings sections and empty API-key state", async () => {
    renderSettings();

    expect(
      await screen.findByRole("heading", { name: "Settings" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/settings is the admin step/i)).toBeInTheDocument();
    const apiKeysTab = screen.getByRole("tab", { name: "API keys" });
    apiKeysTab.focus();
    expect(apiKeysTab).toHaveFocus();
    fireEvent.click(apiKeysTab);

    expect(await screen.findByText("No API keys yet.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create key" })).toBeEnabled();
  });

  it("keeps structured API errors inside the settings surface", async () => {
    vi.mocked(listApiKeys).mockRejectedValue(
      new Error("API keys could not be loaded."),
    );
    renderSettings("/app/settings?tab=api-keys");

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "API keys could not be loaded.",
    );
    expect(
      screen.getByRole("heading", { name: "Settings" }),
    ).toBeInTheDocument();
  });

  it("explains CI key handling and shows a new secret exactly once", async () => {
    vi.mocked(createApiKey).mockResolvedValue({
      api_key: apiKey(),
      secret: "clab_one_time_secret",
    });
    renderSettings("/app/settings?tab=api-keys");

    expect(await screen.findByText("GitHub Actions setup")).toBeInTheDocument();
    expect(screen.getByText("CORPUSLAB_API_KEY")).toBeInTheDocument();
    expect(screen.getByText(/stores only a one-way hash/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Create key" }));

    expect(
      await screen.findByLabelText("Created API key secret"),
    ).toHaveTextContent("clab_one_time_secret");
    expect(screen.getByText(/shown once/i)).toBeInTheDocument();
    expect(createApiKey).toHaveBeenCalledWith({
      name: "GitHub Actions",
      scopes: ["ci_eval_runs"],
    });
  });

  it("shows scope, creation time, and last use without exposing a secret", async () => {
    vi.mocked(listApiKeys).mockResolvedValue([apiKey()]);
    renderSettings("/app/settings?tab=api-keys");

    expect(await screen.findByText("Release gate key")).toBeInTheDocument();
    expect(screen.getByText(/scope ci eval runs/i)).toBeInTheDocument();
    expect(screen.getByText(/^Created /i)).toBeInTheDocument();
    expect(screen.getByText(/^Last used /i)).toBeInTheDocument();
    expect(screen.queryByText(/clab_one_time_secret/i)).not.toBeInTheDocument();
  });
});

function renderSettings(initialEntry = "/app/settings") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <SettingsPage />
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

function currentUser() {
  return {
    user: {
      user: {
        id: "018f7a2a-6e2e-7000-a000-000000004820",
        email: "demo@corpuslab.ai",
        name: "Demo User",
        created_at: "2026-07-04T08:00:00Z",
      },
      organization: {
        id: "018f7a2a-6e2e-7000-a000-000000004821",
        name: "CorpusLab Demo Organization",
        created_at: "2026-07-04T08:00:00Z",
      },
      workspace: {
        id: "018f7a2a-6e2e-7000-a000-000000004822",
        organization_id: "018f7a2a-6e2e-7000-a000-000000004821",
        name: "Corpus Demo Workspace",
        created_at: "2026-07-04T08:00:00Z",
      },
      role: "owner" as const,
    },
  };
}

function apiKey() {
  return {
    id: "018f7a2a-6e2e-7000-a000-000000004830",
    workspace_id: "018f7a2a-6e2e-7000-a000-000000004822",
    name: "Release gate key",
    prefix: "clab_01234567",
    scopes: ["ci_eval_runs" as const],
    created_at: "2026-08-09T08:00:00Z",
    last_used_at: "2026-08-09T09:00:00Z",
    revoked_at: null,
  };
}
