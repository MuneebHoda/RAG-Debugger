import { expect, test, type Page } from "@playwright/test";

const authSession = {
  email: "demo@corpuslab.ai",
  workspaceName: "Corpus Demo Workspace",
  issuedAt: "2026-06-24T00:00:00.000Z",
};

const authResponse = {
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

async function mockCurrentUser(page: Page) {
  await page.route("**/api/v1/auth/me", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: authResponse,
    });
  });
}

async function seedDemoSession(page: Page) {
  await mockCurrentUser(page);
  await page.addInitScript((session) => {
    window.localStorage.setItem(
      "corpuslab.auth.session",
      JSON.stringify(session),
    );
  }, authSession);
}

test("renders the CorpusLab public site", async ({ page }) => {
  await page.goto("/");
  await expect(
    page.getByRole("heading", {
      name: "Turn every corpus into trusted retrieval.",
    }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "Features" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Pricing" })).toBeVisible();
  await expect(
    page.getByAltText(/abstract corpuslab evidence intelligence map/i),
  ).toBeVisible();
});

test("renders pricing and auth pages", async ({ page }) => {
  await page.goto("/pricing");
  await expect(page.getByRole("heading", { name: "Team" })).toBeVisible();
  await expect(page.getByText("$299/mo")).toBeVisible();
  await expect(page.getByText(/platform units/i).first()).toBeVisible();

  await page.goto("/login");
  await mockCurrentUser(page);
  await page.route("**/api/v1/auth/login", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: authResponse,
    });
  });
  await expect(page.getByRole("heading", { name: /sign in/i })).toBeVisible();
  await expect(page.getByText("demo@corpuslab.ai")).toBeVisible();
  await page.getByLabel("Email").fill("demo@corpuslab.ai");
  await page.getByLabel("Password").fill("CorpusLab#2026");
  await page.getByRole("button", { name: /open workbench/i }).click();
  await expect(page).toHaveURL(/\/app$/);

  await page.goto("/signup");
  await expect(
    page.getByRole("heading", { name: /create your corpuslab workspace/i }),
  ).toBeVisible();
});

test("serves generated theme assets and product screenshots", async ({
  page,
}) => {
  for (const file of [
    "corpuslab-hero-theme.png",
    "corpuslab-evidence-map.png",
    "corpuslab-quality-layer.png",
    "corpuslab-dashboard.png",
    "corpuslab-sources.png",
    "corpuslab-retrieval.png",
    "corpuslab-evals.png",
    "corpuslab-reports.png",
  ]) {
    const response = await page.request.get(`/product/${file}`);
    expect(response.ok()).toBeTruthy();
  }
});

test("uploads a sample file and shows chunk preview", async ({ page }) => {
  await seedDemoSession(page);

  const documentId = "018f7a2a-6e2e-7000-a000-000000000001";
  const sourceId = "018f7a2a-6e2e-7000-a000-000000000002";
  const projectId = "018f7a2a-6e2e-7000-a000-000000000003";
  const chunk = {
    id: "018f7a2a-6e2e-7000-a000-000000000004",
    document_id: documentId,
    ordinal: 0,
    text: "Alpha beta",
    token_count: 2,
    byte_range: { start: 0, end: 10 },
    checksum: "1234567890abcdef",
    strategy: "structured",
    section_title: "Projects",
    split_reason: "document_end",
    quality_flags: ["good_evidence_candidate"],
    is_duplicate: false,
    text_density: 0.9,
    evidence_score_hint: 0.8,
  };
  const document = {
    id: documentId,
    source_id: sourceId,
    path: "sample.md",
    mime_type: "text/markdown",
    checksum: "abcdef",
    byte_size: 18,
    profile: "technical_docs",
    extraction_quality: "high",
    warnings: [],
  };
  const source = {
    id: sourceId,
    project_id: projectId,
    name: "Corpus upload",
    kind: { FileSet: { root_hint: "browser-upload" } },
    sync_policy: "Manual",
    chunking: {
      target_tokens: 2,
      overlap_tokens: 0,
      strategy: "structured",
    },
  };

  await page.route("**/api/v1/sources", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: [
        {
          source,
          document_count: 1,
          chunk_count: 1,
          documents: [{ document, chunk_count: 1 }],
        },
      ],
    });
  });
  await page.route("**/api/v1/sources/files", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      status: 201,
      json: {
        source,
        ingestion_run: {
          id: "018f7a2a-6e2e-7000-a000-000000000005",
          source_id: sourceId,
          status: "Completed",
          totals: {
            files_received: 1,
            documents_created: 1,
            chunks_created: 1,
            failed_files: 0,
          },
          started_at: "2026-06-23T00:00:00Z",
          completed_at: "2026-06-23T00:00:01Z",
        },
        documents: [
          {
            file_name: "sample.md",
            status: "success",
            document,
            chunk_count: 1,
            preview_chunks: [chunk],
            error_code: null,
            message: null,
          },
        ],
        totals: {
          files_received: 1,
          documents_created: 1,
          chunks_created: 1,
          failed_files: 0,
        },
      },
    });
  });
  await page.route(
    `**/api/v1/documents/${documentId}/chunks`,
    async (route) => {
      await route.fulfill({
        contentType: "application/json",
        json: [chunk],
      });
    },
  );

  await page.goto("/app/sources");
  await page.getByLabel("Choose files").setInputFiles({
    name: "sample.md",
    mimeType: "text/markdown",
    buffer: Buffer.from("Alpha beta gamma"),
  });
  await page.getByRole("button", { name: "Ingest files" }).click();

  const documentLink = page.getByRole("link", { name: /sample\.md.*1 chunks/ });
  await expect(documentLink).toBeVisible();
  await documentLink.click();
  await expect(page).toHaveURL(new RegExp(`/app/sources/${documentId}$`));
  await expect(page.getByText("Projects", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Structured document", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Alpha beta")).toBeVisible();
});

test("tests retrieval and shows cited evidence", async ({ page }) => {
  await seedDemoSession(page);

  const documentId = "018f7a2a-6e2e-7000-a000-000000000101";
  const sourceId = "018f7a2a-6e2e-7000-a000-000000000102";
  const projectId = "018f7a2a-6e2e-7000-a000-000000000103";
  const chunkId = "018f7a2a-6e2e-7000-a000-000000000104";
  const source = {
    id: sourceId,
    project_id: projectId,
    name: "Corpus upload",
    kind: { FileSet: { root_hint: "browser-upload" } },
    sync_policy: "Manual",
    chunking: {
      target_tokens: 512,
      overlap_tokens: 64,
      strategy: "structured",
    },
  };
  const document = {
    id: documentId,
    source_id: sourceId,
    path: "resume.md",
    mime_type: "text/markdown",
    checksum: "abcdef",
    byte_size: 64,
    profile: "technical_docs",
    extraction_quality: "high",
    warnings: [],
  };
  const chunk = {
    id: chunkId,
    document_id: documentId,
    ordinal: 0,
    text: "Built GPU indexing experiments.",
    token_count: 4,
    byte_range: { start: 0, end: 32 },
    checksum: "1234567890abcdef",
    strategy: "structured",
    section_title: "Projects",
    split_reason: "document_end",
    quality_flags: ["good_evidence_candidate"],
    is_duplicate: false,
    text_density: 0.9,
    evidence_score_hint: 0.8,
  };

  await page.route("**/api/v1/sources", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: [
        {
          source,
          document_count: 1,
          chunk_count: 1,
          documents: [{ document, chunk_count: 1 }],
        },
      ],
    });
  });
  await page.route("**/api/v1/embeddings/status", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: {
        model: {
          provider: "local",
          model_name: "local-hash-v1",
          dimension: 384,
        },
        total_chunks: 1,
        indexed_chunks: 1,
        missing_chunks: 0,
        stale_chunks: 0,
        last_indexed_at: "2026-06-23T00:00:00Z",
      },
    });
  });
  await page.route("**/api/v1/retrieval/evals", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: [],
    });
  });
  await page.route("**/api/v1/retrieval/query", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: {
        run: {
          id: "018f7a2a-6e2e-7000-a000-000000000105",
          query: "gpu indexing",
          top_k: 5,
          retrieval_mode: "hybrid",
          latency_ms: 4,
          created_at: "2026-06-23T00:00:00Z",
        },
        answer: {
          status: "answered",
          text: "Built GPU indexing experiments [1]",
          citations: [
            {
              label: "[1]",
              chunk_id: chunkId,
              document_id: documentId,
              document_path: "resume.md",
              chunk_ordinal: 0,
              section_title: "Projects",
              checksum_prefix: "1234567890ab",
              snippet: "Built GPU indexing experiments",
            },
          ],
        },
        hits: [
          {
            rank: 1,
            score: 3.2,
            chunk,
            document,
            source,
            matched_terms: [
              { term: "gpu", count: 1 },
              { term: "indexing", count: 1 },
            ],
            score_breakdown: {
              semantic: 0.7,
              lexical: 2.5,
              phrase: 0.5,
              section: 0,
              path: 0,
              metadata: 0.1,
            },
            normalized_score_breakdown: {
              semantic: 0.28,
              lexical: 1,
              phrase: 0.2,
              section: 0,
              path: 0,
              metadata: 0.04,
            },
            snippet: "Built GPU indexing experiments",
            citation: {
              label: "[1]",
              chunk_id: chunkId,
              document_id: documentId,
              document_path: "resume.md",
              chunk_ordinal: 0,
              section_title: "Projects",
              checksum_prefix: "1234567890ab",
              snippet: "Built GPU indexing experiments",
            },
            quality_flags: ["semantic_match", "exact_term_match"],
            evidence_strength: "strong",
            duplicate_count: 1,
          },
        ],
        embedding_status: {
          readiness: "ready",
          required: true,
          model: {
            provider: "local",
            model_name: "local-hash-v1",
            dimension: 384,
          },
          total_chunks: 1,
          indexed_chunks: 1,
          missing_chunks: 0,
          stale_chunks: 0,
        },
      },
    });
  });

  await page.goto("/app/retrieval");
  await page.getByLabel("What should the corpus answer?").fill("gpu indexing");
  await page.getByRole("button", { name: "Run retrieval" }).click();

  await expect(
    page.getByText("Built GPU indexing experiments [1]"),
  ).toBeVisible();
  await expect(page.getByText("[1] resume.md · chunk 1")).toBeVisible();
  await expect(page.getByText("gpu × 1")).toBeVisible();
  await expect(page.getByText("Exact term")).toBeVisible();
  await expect(page.getByLabel("Score breakdown")).toBeVisible();
});

test("opens trace debugger and reruns a saved trace", async ({ page }) => {
  await seedDemoSession(page);

  const traceId = "018f7a2a-6e2e-7000-a000-000000000301";
  const sourceId = "018f7a2a-6e2e-7000-a000-000000000302";
  const documentId = "018f7a2a-6e2e-7000-a000-000000000303";
  const chunkId = "018f7a2a-6e2e-7000-a000-000000000304";
  const source = {
    id: sourceId,
    project_id: "018f7a2a-6e2e-7000-a000-000000000305",
    name: "Corpus upload",
    kind: { FileSet: { root_hint: "browser-upload" } },
    sync_policy: "Manual",
    chunking: {
      target_tokens: 512,
      overlap_tokens: 64,
      strategy: "structured",
    },
  };
  const document = {
    id: documentId,
    source_id: sourceId,
    path: "platform-guide.md",
    mime_type: "text/markdown",
    checksum: "abcdef",
    byte_size: 64,
    profile: "technical_docs",
    extraction_quality: "high",
    warnings: [],
  };
  const chunk = {
    id: chunkId,
    document_id: documentId,
    ordinal: 0,
    text: "GPU workers speed up embedding refreshes.",
    token_count: 6,
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
  const retrieval = {
    run: {
      id: "018f7a2a-6e2e-7000-a000-000000000306",
      query: "gpu embedding workers",
      top_k: 5,
      retrieval_mode: "hybrid",
      latency_ms: 8,
      created_at: "2026-06-23T00:00:00Z",
    },
    answer: {
      status: "answered",
      text: "GPU workers speed up embedding refreshes [1]",
      citations: [],
    },
    hits: [
      {
        rank: 1,
        score: 3.4,
        chunk,
        document,
        source,
        matched_terms: [{ term: "gpu", count: 1 }],
        score_breakdown: {
          semantic: 0.9,
          lexical: 1.8,
          phrase: 0.4,
          section: 0.1,
          path: 0,
          metadata: 0.1,
        },
        normalized_score_breakdown: {
          semantic: 0.5,
          lexical: 1,
          phrase: 0.2,
          section: 0.05,
          path: 0,
          metadata: 0.05,
        },
        snippet: "GPU workers speed up embedding refreshes.",
        citation: {
          label: "[1]",
          chunk_id: chunkId,
          document_id: documentId,
          document_path: "platform-guide.md",
          chunk_ordinal: 0,
          section_title: "Indexing",
          checksum_prefix: "1234567890ab",
          snippet: "GPU workers speed up embedding refreshes.",
        },
        quality_flags: ["semantic_match"],
        evidence_strength: "strong",
        duplicate_count: 1,
      },
    ],
    embedding_status: {
      readiness: "ready",
      required: true,
      model: {
        provider: "local",
        model_name: "local-hash-v1",
        dimension: 384,
      },
      total_chunks: 1,
      indexed_chunks: 1,
      missing_chunks: 0,
      stale_chunks: 0,
    },
  };
  const trace = {
    id: traceId,
    project_id: source.project_id,
    input: "gpu embedding workers",
    output: "GPU workers speed up embedding refreshes [1]",
    started_at: "2026-06-23T00:00:00Z",
    completed_at: "2026-06-23T00:00:01Z",
    failure_labels: ["weak_evidence"],
    source_run_id: retrieval.run.id,
    summary: "Retrieved one chunk, but CorpusLab found one quality signal.",
    status: "warning",
    evidence_strength: "strong",
    spans: [
      {
        id: "018f7a2a-6e2e-7000-a000-000000000307",
        kind: "query_input",
        title: "Query input",
        description: "Captured query settings.",
        started_at: "2026-06-23T00:00:00Z",
        completed_at: "2026-06-23T00:00:00Z",
        latency_ms: 0,
        status: "succeeded",
        detail: {
          type: "query_input",
          top_k: 5,
          retrieval_mode: "hybrid",
          source_filter_count: 0,
          document_filter_count: 0,
        },
      },
      {
        id: "018f7a2a-6e2e-7000-a000-000000000308",
        kind: "retrieval",
        title: "Retrieval ranking",
        description: "Scored chunks.",
        started_at: "2026-06-23T00:00:00Z",
        completed_at: "2026-06-23T00:00:01Z",
        latency_ms: 8,
        status: "succeeded",
        detail: {
          type: "retrieval",
          hit_count: 1,
          top_score: 3.4,
          embedding_readiness: "ready",
        },
      },
    ],
    retrieval,
    reruns: [],
  };

  await page.route("**/api/v1/traces", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: [
        {
          id: traceId,
          query: "gpu embedding workers",
          retrieval_mode: "hybrid",
          latency_ms: 8,
          evidence_strength: "strong",
          failure_labels: ["weak_evidence"],
          span_count: 2,
          rerun_count: 0,
          created_at: "2026-06-23T00:00:00Z",
        },
      ],
    });
  });
  await page.route(`**/api/v1/traces/${traceId}/rerun`, async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: {
        trace: {
          ...trace,
          reruns: [
            {
              id: "018f7a2a-6e2e-7000-a000-000000000309",
              request: {
                query: "gpu embedding workers",
                top_k: 3,
                retrieval_mode: "lexical",
                source_ids: [],
                document_ids: [],
              },
              response: {
                ...retrieval,
                run: { ...retrieval.run, retrieval_mode: "lexical", top_k: 3 },
              },
              score_delta: -0.4,
              latency_delta_ms: 2,
              overlap_count: 1,
              changed_rank_count: 0,
              created_at: "2026-06-23T00:00:02Z",
            },
          ],
        },
        comparison: {
          id: "018f7a2a-6e2e-7000-a000-000000000309",
          request: {
            query: "gpu embedding workers",
            top_k: 3,
            retrieval_mode: "lexical",
            source_ids: [],
            document_ids: [],
          },
          response: {
            ...retrieval,
            run: { ...retrieval.run, retrieval_mode: "lexical", top_k: 3 },
          },
          score_delta: -0.4,
          latency_delta_ms: 2,
          overlap_count: 1,
          changed_rank_count: 0,
          created_at: "2026-06-23T00:00:02Z",
        },
      },
    });
  });
  await page.route(`**/api/v1/traces/${traceId}`, async (route) => {
    await route.fulfill({
      contentType: "application/json",
      json: trace,
    });
  });

  await page.goto("/app/traces");
  await expect(page.getByRole("heading", { name: "Runs" })).toBeVisible();
  await page.getByRole("link", { name: /gpu embedding workers/i }).click();
  await expect(page).toHaveURL(new RegExp(`/app/traces/${traceId}$`));
  await expect(page.getByText("What happened")).toBeVisible();

  await page.getByRole("tab", { name: "Evidence" }).click();
  await expect(
    page.getByText("GPU workers speed up embedding refreshes."),
  ).toBeVisible();

  await page.getByRole("tab", { name: "Timeline" }).click();
  await expect(page.getByText("Retrieval ranking")).toBeVisible();

  await page.getByRole("tab", { name: "Compare" }).click();
  await page.getByLabel("Retrieval mode").selectOption("lexical");
  await page.getByLabel("Results to return").fill("3");
  await page.getByRole("button", { name: "Run comparison" }).click();

  await expect(page.getByText("Top-score change")).toBeVisible();
  await expect(page.getByText("-0.40", { exact: false })).toBeVisible();
});

test("workbench stays readable without horizontal overflow", async ({
  page,
}) => {
  await seedDemoSession(page);
  await page.route("**/healthz", (route) =>
    route.fulfill({ contentType: "application/json", json: { status: "ok" } }),
  );
  await page.route("**/api/v1/config", (route) =>
    route.fulfill({
      contentType: "application/json",
      json: {
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
        ui: { api_base_url: "http://127.0.0.1:18080", show_local_badges: true },
      },
    }),
  );
  await page.route("**/api/v1/overview", (route) =>
    route.fulfill({
      contentType: "application/json",
      json: {
        generated_at: "2026-06-27T00:00:00Z",
        health: {
          score: 0,
          status: "needs_documents",
          summary: "Add documents.",
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
      },
    }),
  );
  await page.route("**/api/v1/sources", (route) =>
    route.fulfill({ contentType: "application/json", json: [] }),
  );

  for (const viewport of [
    { width: 1440, height: 900 },
    { width: 768, height: 900 },
    { width: 390, height: 844 },
  ]) {
    await page.setViewportSize(viewport);
    await page.goto("/app/sources");
    await expect(page.getByRole("heading", { name: "Corpus" })).toBeVisible();
    const sizes = await page.evaluate(() => ({
      viewport: document.documentElement.clientWidth,
      content: document.documentElement.scrollWidth,
    }));
    expect(sizes.content).toBeLessThanOrEqual(sizes.viewport);
  }
});

test("completes the real guided workflow against the memory API", async ({
  page,
}) => {
  await page.goto("/login");
  await page.getByLabel("Email").fill("demo@corpuslab.ai");
  await page.getByLabel("Password").fill("CorpusLab#2026");
  await page.getByRole("button", { name: /open workbench/i }).click();
  await expect(page).toHaveURL(/\/app$/);

  await page.goto("/app/sources");
  await page.getByLabel("Choose files").setInputFiles({
    name: "gpu-platform-guide.md",
    mimeType: "text/markdown",
    buffer: Buffer.from(
      "# GPU indexing\n\nGPU workers accelerate embedding indexing and refresh vector search indexes.\n\n# Reliability\n\nQuality gates compare recall and precision before release.",
    ),
  });
  await page.getByRole("button", { name: "Ingest files" }).click();
  await expect(
    page.getByRole("link", { name: /gpu-platform-guide\.md/i }),
  ).toBeVisible();

  await page.goto("/app/retrieval");
  await page.getByText("Advanced", { exact: true }).click();
  await page.getByRole("button", { name: "Index" }).click();
  await expect(page.getByText(/1\/1 indexed/i)).toBeVisible();
  await page
    .getByLabel("What should the corpus answer?")
    .fill("How do GPU workers help indexing?");
  await page.getByRole("button", { name: "Run retrieval" }).click();
  await expect(
    page.getByText(/GPU workers accelerate embedding indexing/i).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Debug this run" }).click();
  await expect(page).toHaveURL(/\/app\/traces\/[0-9a-f-]+$/);
  await expect(page.getByText("What happened")).toBeVisible();

  await page.getByRole("tab", { name: "Compare" }).click();
  await page.getByLabel("Retrieval mode").selectOption("lexical");
  await page.getByRole("button", { name: "Run comparison" }).click();
  await expect(page.getByText("Top-score change")).toBeVisible();

  await page.getByRole("tab", { name: "Summary" }).click();
  await page.getByRole("button", { name: "Choose evidence" }).click();
  await page
    .getByLabel("Quality dataset")
    .selectOption({ label: "Default retrieval dataset" });
  await page.getByRole("checkbox").first().check();
  await page.getByRole("button", { name: "Save quality case" }).click();
  await expect(page.getByText("Quality case saved.")).toBeVisible();
});
