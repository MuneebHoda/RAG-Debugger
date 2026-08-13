import { expect, test } from "@playwright/test";

import {
  expectElementsNotToOverlap,
  expectNoHorizontalOverflow,
} from "./support/layoutAssertions";
import { testCredentials } from "./support/testCredentials";

test("completes the real guided workflow against the memory API", async ({
  page,
}) => {
  const fileName = `gpu-platform-guide-${crypto.randomUUID()}.md`;

  await page.goto("/login");
  await page.getByLabel("Email").fill(testCredentials.email);
  await page.getByLabel("Password").fill(testCredentials.password);
  await page.getByRole("button", { name: /open workbench/i }).click();
  await expect(page).toHaveURL(/\/app$/);

  await page.goto("/app/sources");
  await page.getByLabel("Choose files").setInputFiles({
    name: fileName,
    mimeType: "text/markdown",
    buffer: Buffer.from(
      "# GPU indexing\n\nGPU workers accelerate embedding indexing and refresh vector search indexes.\n\n# Reliability\n\nQuality gates compare recall and precision before release.",
    ),
  });
  await page.getByRole("button", { name: "Ingest files" }).click();
  await expect(
    page.getByRole("link").filter({ hasText: fileName }),
  ).toBeVisible();

  await page.goto("/app/retrieval");
  await page.getByText("Advanced", { exact: true }).click();
  await page.getByRole("button", { name: "Index", exact: true }).click();
  await expect(page.getByText(/indexed · local-hash-v1/i)).toBeVisible();
  await page
    .getByLabel("What should the corpus answer?")
    .fill("How do GPU workers help indexing?");
  await page.getByRole("button", { name: "Run retrieval" }).click();
  await expect(
    page.getByText(/GPU workers accelerate embedding indexing/i).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Debug this run" }).click();
  await expect(page).toHaveURL(/\/app\/traces\/[0-9a-f-]+$/);
  await expect(
    page.getByText("Answer support: Supported", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText(/^Retrieval quality: /)).toBeVisible();

  await page.getByRole("tab", { name: "Compare" }).click();
  await page.getByLabel("Retrieval mode").selectOption("lexical");
  await page.getByRole("button", { name: "Run comparison" }).click();
  await expect(page.getByText("Top-score change")).toBeVisible();

  await page.getByRole("tab", { name: "Summary" }).click();
  await page.getByRole("button", { name: "Choose evidence" }).click();
  await page
    .getByLabel("Quality dataset")
    .selectOption({ label: "Default retrieval dataset" });
  const searchInput = page.getByLabel("Search corpus evidence");
  await searchInput.fill("indexing");
  await searchInput.press("Enter");
  await expect(
    page.getByText(/document results and .* chunk results/i),
  ).toBeVisible();

  await page.setViewportSize({ width: 390, height: 844 });
  const resultGrid = page.getByLabel("Evidence search results");
  await expect(resultGrid).toBeVisible();
  expect(
    await resultGrid.evaluate(
      (element) =>
        getComputedStyle(element).gridTemplateColumns.split(" ").length,
    ),
  ).toBe(1);
  await expectNoHorizontalOverflow(page);
  await page.setViewportSize({ width: 1440, height: 900 });

  const candidateEvidence = page.getByRole("region", {
    name: "Retrieved evidence from this run",
  });
  const exactChunk = candidateEvidence.getByRole("button", {
    name: "Expect this exact chunk",
  });
  await exactChunk.focus();
  await page.keyboard.press("Enter");
  await expect(exactChunk).toHaveAttribute("aria-pressed", "true");
  const wholeDocument = candidateEvidence.getByRole("button", {
    name: "Accept evidence from this document",
  });
  await wholeDocument.focus();
  await page.keyboard.press("Enter");
  await expect(wholeDocument).toHaveAttribute("aria-pressed", "true");

  await exactChunk.focus();
  await page.keyboard.press("Enter");
  await expect(exactChunk).toHaveAttribute("aria-pressed", "false");
  await page.keyboard.press("Enter");
  await expect(exactChunk).toHaveAttribute("aria-pressed", "true");

  const clearEvidence = page.getByRole("button", {
    name: "Clear all selected evidence",
  });
  await clearEvidence.focus();
  await page.keyboard.press("Enter");
  await expect(exactChunk).toHaveAttribute("aria-pressed", "false");
  await expect(wholeDocument).toHaveAttribute("aria-pressed", "false");
  await exactChunk.focus();
  await page.keyboard.press("Enter");
  await wholeDocument.focus();
  await page.keyboard.press("Enter");
  await expect(exactChunk).toHaveAttribute("aria-pressed", "true");
  await expect(wholeDocument).toHaveAttribute("aria-pressed", "true");

  await expect(page.getByText("Selected expected evidence")).toBeVisible();
  await expect(page.getByText(/Exact chunk expectation/i)).toBeVisible();
  await expect(page.getByText(/Document-level expectation/i)).toBeVisible();
  const saveQualityCase = page.getByRole("button", {
    name: "Save quality case",
  });
  await saveQualityCase.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByText("Quality case saved.")).toBeVisible();

  await page.goto("/app/evals");
  await page
    .locator('a[href^="/app/evals/datasets/"]')
    .filter({ hasText: "Default retrieval dataset" })
    .click();
  await expect(
    page.getByRole("heading", { name: "Run an experiment" }),
  ).toBeVisible();
  await expect(
    page.getByText("How do GPU workers help indexing?").first(),
  ).toBeVisible();
  await expect(page.getByText("Expected exact chunk").first()).toBeVisible();
  await expect(page.getByText("Expected document").first()).toBeVisible();

  const apiUrl = "http://127.0.0.1:18080/api/v1";
  const sourceDatasetId = page.url().split("/").at(-1);
  expect(sourceDatasetId).toBeTruthy();
  const sourceDatasetResponse = await page.request.get(
    `${apiUrl}/eval-lab/datasets/${sourceDatasetId}`,
  );
  expect(sourceDatasetResponse.ok()).toBeTruthy();
  const sourceDataset = (await sourceDatasetResponse.json()) as {
    cases: Array<{ expected_chunk_ids: string[] }>;
  };
  const expectedChunkId = sourceDataset.cases.at(-1)?.expected_chunk_ids[0];
  expect(expectedChunkId).toBeTruthy();

  const ciDatasetName = `CI release gate ${crypto.randomUUID()}`;
  const createDatasetResponse = await page.request.post(
    `${apiUrl}/eval-lab/datasets`,
    { data: { name: ciDatasetName, description: "Playwright CI gate" } },
  );
  expect(createDatasetResponse.ok()).toBeTruthy();
  const ciDataset = (await createDatasetResponse.json()) as { id: string };
  const createCaseResponse = await page.request.post(
    `${apiUrl}/eval-lab/datasets/${ciDataset.id}/cases`,
    {
      data: {
        name: "Missing release evidence",
        query: "qxzv blorp",
        top_k: 5,
        expected_chunk_ids: [expectedChunkId],
      },
    },
  );
  expect(createCaseResponse.ok()).toBeTruthy();

  await page.goto("/app/settings?tab=api-keys");
  const keyName = `Playwright CI ${crypto.randomUUID()}`;
  await page.getByLabel("Key name").fill(keyName);
  await page.getByRole("button", { name: "Create key" }).click();
  const secretRegion = page.getByLabel("Created API key secret");
  await expect(secretRegion).toContainText("shown once");
  await expect(page.getByText("CORPUSLAB_API_KEY")).toBeVisible();
  const apiKey = await secretRegion.locator("code").textContent();
  expect(apiKey).toBeTruthy();

  const ciResponse = await page.request.post(`${apiUrl}/eval-lab/ci/runs`, {
    data: {
      dataset_id: ciDataset.id,
      name: "Playwright release gate",
      modes: ["lexical"],
      branch: "feature/ci-polish",
      commit_sha: "abc123def456",
      base_ref: "main",
      head_ref: "feature/ci-polish",
      config_label: "playwright",
      fail_on_gate: true,
    },
    headers: { Authorization: `Bearer ${apiKey}` },
  });
  expect(ciResponse.status()).toBe(422);
  const ciRun = (await ciResponse.json()) as { id: string };

  await page.goto("/app/evals?view=ci-runs");
  await page
    .getByRole("link", { name: `Open CI run for ${ciDatasetName}` })
    .click();
  await expect(page).toHaveURL(`/app/evals/ci-runs/${ciRun.id}`);
  await expect(
    page.getByRole("heading", { name: "Gate failed" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Run metadata" }),
  ).toBeVisible();
  await expect(page.getByText("feature/ci-polish").first()).toBeVisible();
  await expect(page.getByText("abc123def456")).toBeVisible();
  await expect(page.getByText("playwright").first()).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Failed metrics" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Failed cases" }),
  ).toBeVisible();
  await expect(page.getByText("qxzv blorp").first()).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Metrics summary" }),
  ).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await expectNoHorizontalOverflow(page);
  await page.setViewportSize({ width: 1440, height: 900 });

  await page.getByRole("button", { name: "Create audit report" }).click();
  await expect(page.getByLabel("Privacy")).toHaveValue("metadata_only");
  await page.getByRole("button", { name: "Create report" }).click();
  await expect(page).toHaveURL(/\/app\/reports\/[0-9a-f-]+$/);
  await expect(page.getByRole("heading", { name: /audit/i })).toBeVisible();

  await page.goto(`/app/evals/datasets/${sourceDatasetId}`);

  for (const viewport of [
    { width: 1440, height: 900 },
    { width: 1024, height: 900 },
    { width: 768, height: 900 },
    { width: 390, height: 844 },
  ]) {
    await page.setViewportSize(viewport);
    const heading = page.getByRole("heading", { name: "Run an experiment" });
    const action = page.getByRole("button", { name: "Run experiment" });
    await expect(heading).toBeVisible();
    await expect(action).toBeVisible();
    await expectElementsNotToOverlap(heading, action);
    await expectElementsNotToOverlap(
      page.getByLabel("Results per question"),
      action,
    );
    expect(
      await action.evaluate(
        (element) => element.scrollWidth <= element.clientWidth,
      ),
    ).toBeTruthy();
  }
});

test("ingests privacy-scoped external traces through Debugger and permitted reports", async ({
  page,
}) => {
  test.setTimeout(120_000);
  const apiUrl = "http://127.0.0.1:18080/api/v1";
  const suffix = crypto.randomUUID();

  await page.goto("/login");
  await page.getByLabel("Email").fill(testCredentials.email);
  await page.getByLabel("Password").fill(testCredentials.password);
  await page.getByRole("button", { name: /open workbench/i }).click();
  await expect(page).toHaveURL(/\/app$/);

  await page.goto("/app/sources");
  await page.getByLabel("Choose files").setInputFiles({
    name: `collector-guide-${suffix}.md`,
    mimeType: "text/markdown",
    buffer: Buffer.from(
      "# Collector reliability\n\nCollector workers validate trace batches before publishing them to the local index.",
    ),
  });
  await page.getByRole("button", { name: "Ingest files" }).click();
  await expect(
    page.getByRole("link").filter({
      hasText: `collector-guide-${suffix}.md`,
    }),
  ).toBeVisible();
  await page.goto("/app/retrieval");
  await page.getByText("Advanced", { exact: true }).click();
  await page.getByRole("button", { name: "Index", exact: true }).click();
  await expect(page.getByText(/indexed · local-hash-v1/i)).toBeVisible();

  const projectResponse = await page.request.get(`${apiUrl}/projects/current`);
  expect(projectResponse.ok()).toBeTruthy();
  const project = (await projectResponse.json()) as { id: string };

  await page.goto("/app/settings?tab=api-keys");
  await expect(page.getByText(project.id)).toBeVisible();
  await page.getByLabel("Key name").fill(`Trace collector ${suffix}`);
  await page.getByLabel("Key purpose").selectOption("trace_ingest");
  await page.getByRole("button", { name: "Create key" }).click();
  const secretRegion = page.getByLabel("Created API key secret");
  const apiKey = await secretRegion.locator("code").textContent();
  expect(apiKey).toBeTruthy();

  const ingest = async (
    externalTraceId: string,
    privacyMode: "metadata_only" | "snippets_allowed" | "full_local_only",
  ) => {
    const response = await page.request.post(`${apiUrl}/traces/ingest`, {
      headers: { Authorization: `Bearer ${apiKey}` },
      data: {
        schema_version: "1",
        project_id: project.id,
        external_trace_id: externalTraceId,
        privacy_mode: privacyMode,
        query: "When do collector workers publish a trace batch?",
        prompt: "SECRET_PLAYWRIGHT_PROMPT",
        answer: "After validation.",
        retrieval_mode: "hybrid",
        top_k: 1,
        failure_labels: ["weak_evidence"],
        retrieved_evidence: [
          {
            external_chunk_id: "collector-evidence-1",
            document_label: "collector-guide.md",
            rank: 1,
            score: 0.82,
            citation_label: "E1",
            snippet:
              "Collector workers validate trace batches before publishing them.",
            answer_support_status: "supported",
            answer_support_reason: "direct_body_support",
          },
        ],
        spans: [
          {
            external_span_id: "retrieve-1",
            operation: "retrieval",
            name: "Retrieve local evidence",
            started_at: "2026-08-12T08:00:00Z",
            completed_at: "2026-08-12T08:00:00.010Z",
            latency_ms: 10,
            status: "succeeded",
          },
          {
            external_span_id: "generate-1",
            parent_span_id: "retrieve-1",
            operation: "generation",
            name: "Generate answer",
            started_at: "2026-08-12T08:00:00.010Z",
            completed_at: "2026-08-12T08:00:00.020Z",
            latency_ms: 10,
            status: "warning",
          },
        ],
      },
    });
    expect(response.status()).toBe(201);
    return (await response.json()) as { trace_id: string };
  };

  const imported = await ingest(
    `playwright-snippets-${suffix}`,
    "snippets_allowed",
  );
  await page.goto(`/app/traces/${imported.trace_id}`);
  await expect(
    page.getByRole("heading", {
      name: "Query withheld by privacy policy",
    }),
  ).toBeVisible();
  await expect(
    page.getByText("snippets allowed", { exact: true }),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Timeline" }).click();
  const hierarchy = page.getByRole("list", { name: "Imported span hierarchy" });
  await expect(hierarchy.getByText("Retrieval", { exact: true })).toBeVisible();
  await expect(
    hierarchy.getByText("Generation", { exact: true }),
  ).toBeVisible();
  await expect(hierarchy.getByText("unspecified")).toHaveCount(2);

  await page.getByRole("tab", { name: "Evidence" }).click();
  await expect(
    page.getByText(
      "Collector workers validate trace batches before publishing them.",
    ),
  ).toBeVisible();
  await expect(page.getByText("collector-guide.md")).toHaveCount(0);

  await page.getByRole("tab", { name: "Summary" }).click();
  await expect(
    page.getByText(/this imported trace cannot become an Eval Lab case/i),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Choose evidence" }),
  ).toHaveCount(0);

  await page.getByRole("button", { name: "Create audit report" }).click();
  await page
    .getByLabel("Privacy", { exact: true })
    .selectOption("snippets_allowed");
  await page.getByRole("button", { name: "Create report" }).click();
  await expect(page).toHaveURL(/\/app\/reports\/[0-9a-f-]+$/);
  await expect(
    page.getByRole("heading", { name: /imported rag trace audit/i }),
  ).toBeVisible();

  const metadata = await ingest(
    `playwright-metadata-${suffix}`,
    "metadata_only",
  );
  await page.goto(`/app/traces/${metadata.trace_id}`);
  await expect(
    page.getByRole("heading", { name: "Query withheld by privacy policy" }),
  ).toBeVisible();
  await expect(page.getByText(/query was not retained/i)).toBeVisible();
  await expect(page.getByText("SECRET_PLAYWRIGHT_PROMPT")).toHaveCount(0);

  const fullLocal = await ingest(
    `playwright-full-${suffix}`,
    "full_local_only",
  );
  await page.goto(`/app/traces/${fullLocal.trace_id}`);
  await expect(
    page.getByRole("button", { name: "Create audit report" }),
  ).toBeDisabled();
  await expect(
    page.getByText(
      /full-local imported traces cannot be reported or exported/i,
    ),
  ).toBeVisible();
  await expect(
    page.getByText(/privacy classification cannot yet be preserved/i),
  ).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await expectNoHorizontalOverflow(page);
});

test("completes the versioned sample demo through Markdown audit export", async ({
  page,
}) => {
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/login");
  await page.getByLabel("Email").fill(testCredentials.email);
  await page.getByLabel("Password").fill(testCredentials.password);
  await page.getByRole("button", { name: /open workbench/i }).click();
  await expect(page).toHaveURL(/\/app$/);

  const loadButton = page.getByRole("button", { name: "Load sample corpus" });
  await expect(loadButton).toBeVisible();
  await loadButton.click();
  const indexButton = page.getByRole("button", { name: "Index sample" });
  await expect(indexButton).toBeVisible();
  await indexButton.click();

  const recommendedQuery = page.getByRole("link", {
    name: "Test recommended query",
  });
  await expect(recommendedQuery).toBeVisible();
  await recommendedQuery.click();
  await expect(page).toHaveURL(/demo_query=account_recovery/);
  await expect(
    page.getByLabel("What should the corpus answer?"),
  ).not.toHaveValue("");
  await page.getByText("Advanced", { exact: true }).click();
  await expect(
    page.getByRole("checkbox", { name: /CorpusLab Sample Corpus/i }),
  ).toBeChecked();
  await page.getByRole("button", { name: "Run retrieval" }).click();
  await expect(
    page.getByRole("heading", { name: "Evidence Summary" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Debug this run" }).click();
  await expect(page).toHaveURL(/\/app\/traces\/[0-9a-f-]+$/);

  await page.getByRole("button", { name: "Create audit report" }).click();
  await expect(page.getByLabel("Privacy")).toHaveValue("metadata_only");
  await page.getByRole("button", { name: "Create report" }).click();
  await expect(page).toHaveURL(/\/app\/reports\/[0-9a-f-]+$/);
  await page.getByRole("button", { name: "Copy Markdown" }).click();
  await expect(page.getByRole("button", { name: "Copied" })).toBeVisible();

  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download Markdown" }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toMatch(/^corpuslab-report-.*\.md$/);

  for (const viewport of [
    { width: 1440, height: 900 },
    { width: 768, height: 900 },
    { width: 390, height: 844 },
  ]) {
    await page.setViewportSize(viewport);
    await expect(page.getByRole("heading", { name: /audit/i })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  }
});
