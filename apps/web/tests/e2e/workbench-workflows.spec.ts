import { expect, test } from "@playwright/test";

import { authResponse, seedDemoSession } from "./support/auth";
import { expectNoHorizontalOverflow } from "./support/layoutAssertions";

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
  const weakChunkId = "018f7a2a-6e2e-7000-a000-000000000311";
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
    byte_size: 100,
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
  const citation = {
    label: "[1]",
    chunk_id: chunkId,
    document_id: documentId,
    document_path: "platform-guide.md",
    chunk_ordinal: 0,
    section_title: "Indexing",
    checksum_prefix: "1234567890ab",
    snippet: "GPU workers speed up embedding refreshes.",
  };
  const weakChunk = {
    ...chunk,
    id: weakChunkId,
    ordinal: 1,
    text: "GPU worker overview without direct implementation detail.",
    token_count: 7,
    byte_range: { start: 43, end: 100 },
    checksum: "fedcba0987654321",
    quality_flags: [],
    evidence_score_hint: 0.2,
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
      citations: [citation],
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
        citation,
        quality_flags: ["semantic_match"],
        evidence_strength: "strong",
        duplicate_count: 1,
      },
      {
        rank: 2,
        score: 0.4,
        chunk: weakChunk,
        document,
        source,
        matched_terms: [{ term: "gpu", count: 1 }],
        score_breakdown: {
          semantic: 0.2,
          lexical: 0.2,
          phrase: 0,
          section: 0,
          path: 0,
          metadata: 0,
        },
        normalized_score_breakdown: {
          semantic: 0.1,
          lexical: 0.1,
          phrase: 0,
          section: 0,
          path: 0,
          metadata: 0,
        },
        snippet: weakChunk.text,
        citation: {
          ...citation,
          label: "[2]",
          chunk_id: weakChunkId,
          chunk_ordinal: 1,
          checksum_prefix: "fedcba098765",
          snippet: weakChunk.text,
        },
        quality_flags: [],
        evidence_strength: "weak",
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
      total_chunks: 2,
      indexed_chunks: 2,
      missing_chunks: 0,
      stale_chunks: 0,
    },
    diagnosis: {
      outcome: "mixed",
      summary:
        "The answer is supported by direct body evidence. Retrieval quality is mixed because some candidates have diagnostic warnings.",
      primary_issue: null,
      failures: [
        {
          code: "weak_evidence",
          severity: "warning",
          title: "Evidence needs review",
          summary: "The saved run contains a retrieval quality signal.",
          evidence_refs: ["E2"],
        },
      ],
      score_explanations: [],
      recommendations: [
        {
          code: "broaden_candidate_pool",
          priority: "high",
          area: "top_k",
          title: "Broaden the candidate pool",
          rationale: "Evidence needs review.",
          action: "Increase top_k and compare the resulting evidence.",
          failure_codes: ["weak_evidence"],
          evidence_refs: ["E2"],
        },
      ],
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
    summary: "Retrieved two chunks, but CorpusLab found one quality signal.",
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
          hit_count: 2,
          top_score: 3.4,
          embedding_readiness: "ready",
        },
      },
    ],
    retrieval,
    reruns: [],
    diagnosis: retrieval.diagnosis,
  };
  const reportId = "018f7a2a-6e2e-7000-a000-000000000310";
  const report = {
    id: reportId,
    workspace_id: authResponse.user.workspace.id,
    project_id: source.project_id,
    title: "Trace retrieval audit",
    subject: "",
    source: { type: "trace", trace_id: traceId },
    privacy_mode: "metadata_only",
    executive_summary: "The trace contains one retrieval quality signal.",
    context: { retrieval_mode: "hybrid", top_k: "5" },
    findings: [],
    recommendations: [],
    evidence: [],
    created_at: "2026-06-23T00:00:03Z",
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
  await page.route("**/api/v1/reports/from-trace", async (route) => {
    await route.fulfill({
      status: 201,
      contentType: "application/json",
      json: report,
    });
  });

  await page.goto("/app/traces");
  await expect(
    page.getByRole("heading", { name: "Trace Debugger" }),
  ).toBeVisible();
  await page.getByRole("link", { name: /gpu embedding workers/i }).click();
  await expect(page).toHaveURL(new RegExp(`/app/traces/${traceId}$`));
  await expect(
    page.getByText("Answer support: Supported", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText(/^Retrieval quality: Mixed$/i)).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Candidate warnings detected" }),
  ).toBeVisible();
  await expect(page.getByText(/Primary issue/i)).toHaveCount(0);

  await page.getByRole("tab", { name: "Evidence" }).click();
  await expect(
    page.getByText("GPU workers speed up embedding refreshes."),
  ).toBeVisible();
  await expect(
    page.getByText("GPU worker overview without direct implementation detail."),
  ).toBeVisible();

  await page.getByRole("tab", { name: "Timeline" }).click();
  await expect(page.getByText("Retrieval ranking")).toBeVisible();

  await page.getByRole("tab", { name: "Compare" }).click();
  await page.getByLabel("Retrieval mode").selectOption("lexical");
  await page.getByLabel("Results to return").fill("3");
  await page.getByRole("button", { name: "Run comparison" }).click();

  await expect(page.getByText("Top-score change")).toBeVisible();
  await expect(page.getByText("-0.40", { exact: false })).toBeVisible();

  await page.getByRole("button", { name: "Create audit report" }).click();
  await expect(page.getByLabel("Privacy")).toHaveValue("metadata_only");
  await page.getByRole("button", { name: "Create report" }).click();
  await expect(page).toHaveURL(new RegExp(`/app/reports/${reportId}$`));
  await expect(page.getByText(report.executive_summary)).toBeVisible();
});

test("creates and opens a privacy-classified audit report", async ({
  page,
}) => {
  await seedDemoSession(page);
  const reportId = "018f7a2a-6e2e-7000-a000-000000000801";
  const traceId = "018f7a2a-6e2e-7000-a000-000000000802";
  const report = {
    id: reportId,
    workspace_id: authResponse.user.workspace.id,
    project_id: "018f7a2a-6e2e-7000-a000-000000000803",
    title: "Retrieval audit",
    subject: "",
    source: { type: "trace", trace_id: traceId },
    privacy_mode: "metadata_only",
    executive_summary: "The run returned weak evidence.",
    context: { retrieval_mode: "hybrid", top_k: "5" },
    findings: [
      {
        code: "weak-evidence",
        severity: "warning",
        title: "Weak evidence",
        summary: "The strongest result did not clear the evidence threshold.",
        failure_labels: ["weak_evidence"],
        evidence_refs: ["E1"],
      },
    ],
    evidence: [],
    recommendations: [
      {
        code: "increase-top-k",
        priority: "high",
        area: "top_k",
        title: "Increase retrieval depth",
        rationale: "Relevant evidence may rank below the cutoff.",
        action: "Rerun with a higher top_k.",
        finding_codes: ["weak-evidence"],
      },
    ],
    created_at: "2026-06-30T12:00:00Z",
  };

  await page.route("**/api/v1/reports/from-trace", (route) =>
    route.fulfill({
      status: 201,
      contentType: "application/json",
      json: report,
    }),
  );
  await page.route(`**/api/v1/reports/${reportId}`, (route) =>
    route.fulfill({ contentType: "application/json", json: report }),
  );
  await page.route("**/api/v1/reports", (route) =>
    route.fulfill({ contentType: "application/json", json: [] }),
  );
  await page.route("**/api/v1/traces", (route) =>
    route.fulfill({
      contentType: "application/json",
      json: [
        {
          id: traceId,
          query: "How are GPU workers configured?",
          retrieval_mode: "hybrid",
          latency_ms: 8,
          evidence_strength: "weak",
          failure_labels: ["weak_evidence"],
          span_count: 4,
          rerun_count: 0,
          created_at: "2026-06-30T11:00:00Z",
        },
      ],
    }),
  );
  await page.route("**/api/v1/eval-lab/experiments", (route) =>
    route.fulfill({ contentType: "application/json", json: [] }),
  );
  await page.route("**/api/v1/eval-lab/ci/runs", (route) =>
    route.fulfill({ contentType: "application/json", json: [] }),
  );
  await page.route("**/api/v1/sources", (route) =>
    route.fulfill({ contentType: "application/json", json: [] }),
  );

  await page.setViewportSize({ width: 1024, height: 900 });
  await page.goto("/app/reports");
  await expect(
    page.getByRole("heading", { name: "Audit Reports" }),
  ).toBeVisible();
  const runSelect = page.getByLabel("Run", { exact: true });
  await expect(runSelect).toBeEnabled();
  await runSelect.focus();
  await expect(runSelect).toBeFocused();
  await runSelect.selectOption(traceId);
  await page.getByRole("button", { name: "Create report" }).click();

  await expect(page).toHaveURL(new RegExp(`/app/reports/${reportId}$`));
  await expect(page.getByText(report.executive_summary)).toBeVisible();
  await expect(page.getByText("Increase retrieval depth")).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.setViewportSize({ width: 390, height: 844 });
  await expectNoHorizontalOverflow(page);
  await expect(
    page.getByRole("button", { name: "Copy Markdown" }),
  ).toBeVisible();
});
