import type { Page } from "@playwright/test";

export const authSession = {
  email: "demo@corpuslab.ai",
  workspaceName: "Corpus Demo Workspace",
  issuedAt: "2026-06-24T00:00:00.000Z",
};

export const authResponse = {
  user: {
    user: {
      id: "018f7a2a-6e2e-7000-a000-000000000901",
      email: authSession.email,
      name: "Demo User",
      created_at: "2026-06-24T00:00:00Z",
    },
    organization: {
      id: "018f7a2a-6e2e-7000-a000-000000000902",
      name: "CorpusLab Demo Organization",
      created_at: "2026-06-24T00:00:00Z",
    },
    workspace: {
      id: "018f7a2a-6e2e-7000-a000-000000000903",
      organization_id: "018f7a2a-6e2e-7000-a000-000000000902",
      name: authSession.workspaceName,
      created_at: "2026-06-24T00:00:00Z",
    },
    role: "owner",
  },
};

export async function mockCurrentUser(page: Page) {
  await page.route("**/api/v1/auth/me", (route) =>
    route.fulfill({ contentType: "application/json", json: authResponse }),
  );
}

export async function seedDemoSession(page: Page) {
  await mockCurrentUser(page);
  await page.route("**/api/v1/demo", (route) =>
    route.fulfill({
      contentType: "application/json",
      json: {
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
      },
    }),
  );
  await page.addInitScript((session) => {
    window.localStorage.setItem(
      "corpuslab.auth.session",
      JSON.stringify(session),
    );
  }, authSession);
}
