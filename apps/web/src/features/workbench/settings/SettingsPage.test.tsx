import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  createApiKey,
  listApiKeys,
  revokeApiKey,
} from "../../../lib/api/apiKeys";
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

  afterEach(() => {
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

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
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", {
      ...navigator,
      clipboard: { writeText },
    });
    vi.mocked(createApiKey).mockResolvedValue({
      api_key: apiKey(),
      secret: "clab_one_time_secret",
    });
    renderSettings("/app/settings?tab=api-keys");

    expect(await screen.findByText("GitHub Actions setup")).toBeInTheDocument();
    expect(screen.getByText("CORPUSLAB_API_KEY")).toBeInTheDocument();
    expect(screen.getByText(/stores only a one-way hash/i)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Key name"), {
      target: { value: "  Release candidate  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create key" }));

    expect(
      await screen.findByLabelText("Created API key secret"),
    ).toHaveTextContent("clab_one_time_secret");
    expect(screen.getByText(/shown once/i)).toBeInTheDocument();
    expect(createApiKey).toHaveBeenCalledWith({
      name: "Release candidate",
      scopes: ["ci_eval_runs"],
    });
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    expect(
      await screen.findByRole("button", { name: "Copied" }),
    ).toBeInTheDocument();
    expect(writeText).toHaveBeenCalledWith("clab_one_time_secret");
  });

  it("shows scope, creation time, and last use without exposing a secret", async () => {
    vi.mocked(listApiKeys).mockResolvedValue([
      apiKey(),
      {
        ...apiKey(),
        id: "018f7a2a-6e2e-7000-a000-000000004831",
        name: "New CI key",
        last_used_at: null,
      },
    ]);
    renderSettings("/app/settings?tab=api-keys");

    expect(await screen.findByText("Release gate key")).toBeInTheDocument();
    expect(screen.getAllByText(/scope ci eval runs/i)).toHaveLength(2);
    expect(screen.getAllByText(/^Created /i)).toHaveLength(2);
    expect(screen.getByText(/^Last used /i)).toBeInTheDocument();
    expect(screen.getByText("Not used yet")).toBeInTheDocument();
    expect(screen.queryByText(/clab_one_time_secret/i)).not.toBeInTheDocument();
  });

  it("does not claim a secret was copied when the clipboard rejects it", async () => {
    const writeText = vi.fn().mockRejectedValue(new Error("Clipboard denied"));
    vi.stubGlobal("navigator", {
      ...navigator,
      clipboard: { writeText },
    });
    vi.mocked(createApiKey).mockResolvedValue({
      api_key: apiKey(),
      secret: "clab_one_time_secret",
    });
    renderSettings("/app/settings?tab=api-keys");

    fireEvent.click(screen.getByRole("button", { name: "Create key" }));
    await screen.findByLabelText("Created API key secret");
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));

    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith("clab_one_time_secret"),
    );
    expect(screen.getByRole("button", { name: "Copy" })).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "The API key secret could not be copied. Copy it manually.",
    );
  });

  it("tells the user to copy manually when clipboard access is unavailable", async () => {
    vi.stubGlobal("navigator", { ...navigator, clipboard: undefined });
    vi.mocked(createApiKey).mockResolvedValue({
      api_key: apiKey(),
      secret: "clab_one_time_secret",
    });
    renderSettings("/app/settings?tab=api-keys");

    fireEvent.click(screen.getByRole("button", { name: "Create key" }));
    await screen.findByLabelText("Created API key secret");
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Clipboard access is unavailable. Copy the secret manually.",
    );
  });

  it("shows API-key creation errors without leaving Settings", async () => {
    vi.mocked(createApiKey).mockRejectedValue(
      new Error("API key could not be created."),
    );
    renderSettings("/app/settings?tab=api-keys");

    fireEvent.click(screen.getByRole("button", { name: "Create key" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "API key could not be created.",
    );
    expect(
      screen.getByRole("heading", { name: "Settings" }),
    ).toBeInTheDocument();
  });

  it("revokes an active key and refreshes its workspace-scoped status", async () => {
    const activeKey = apiKey();
    vi.mocked(listApiKeys)
      .mockResolvedValueOnce([activeKey])
      .mockResolvedValueOnce([
        { ...activeKey, revoked_at: "2026-08-09T10:00:00Z" },
      ]);
    vi.mocked(revokeApiKey).mockResolvedValue(undefined);
    renderSettings("/app/settings?tab=api-keys");

    fireEvent.click(
      await screen.findByRole("button", { name: "Revoke Release gate key" }),
    );

    await waitFor(() =>
      expect(revokeApiKey).toHaveBeenCalledWith(activeKey.id),
    );
    expect(await screen.findByText(/^Revoked /)).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Revoke Release gate key" }),
    ).not.toBeInTheDocument();
  });

  it("keeps a failed key revocation visible and retryable", async () => {
    vi.mocked(listApiKeys).mockResolvedValue([apiKey()]);
    vi.mocked(revokeApiKey).mockRejectedValue(
      new Error("API key could not be revoked."),
    );
    renderSettings("/app/settings?tab=api-keys");

    fireEvent.click(
      await screen.findByRole("button", { name: "Revoke Release gate key" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "API key could not be revoked.",
    );
    expect(
      screen.getByRole("button", { name: "Revoke Release gate key" }),
    ).toBeEnabled();
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
