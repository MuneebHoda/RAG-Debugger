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
  await page.getByRole("button", { name: "Index" }).click();
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
  await expect(page.getByText("Primary diagnosis")).toBeVisible();

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
