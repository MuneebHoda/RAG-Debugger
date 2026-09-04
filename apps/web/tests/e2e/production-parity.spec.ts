import { expect, test } from "@playwright/test";

import { testCredentials } from "./support/testCredentials";

test("runs the packaged guided workflow from login through report", async ({
  page,
}) => {
  await page.goto("/login");

  const runtimeConfig = await page.evaluate(() => window.CORPUSLAB);
  expect(runtimeConfig).toEqual({
    apiBaseUrl: "http://127.0.0.1:18080",
    environment: "test",
    releaseSha: expect.stringMatching(/^[0-9a-f]{40}$/),
  });

  await page.getByLabel("Email").fill(testCredentials.email);
  await page.getByLabel("Password").fill(testCredentials.password);
  await page.getByRole("button", { name: /open workbench/i }).click();
  await expect(page).toHaveURL(/\/app$/);

  await page.getByRole("button", { name: "Load sample corpus" }).click();
  await page.getByRole("button", { name: "Index sample" }).click();
  await page.getByRole("link", { name: /test recommended query/i }).click();
  await expect(
    page.getByLabel("What should the corpus answer?"),
  ).not.toHaveValue("");
  await page.getByRole("button", { name: "Run retrieval" }).click();
  await expect(page.getByText(/^Retrieval quality: /)).toBeVisible();
  await page.getByRole("button", { name: "Debug this run" }).click();
  await expect(page).toHaveURL(/\/app\/traces\/[0-9a-f-]+$/);

  await page.getByRole("button", { name: "Create audit report" }).click();
  await expect(page.getByLabel("Privacy")).toHaveValue("metadata_only");
  await page.getByRole("button", { name: "Create report" }).click();
  await expect(page).toHaveURL(/\/app\/reports\/[0-9a-f-]+$/);
  await expect(page.getByRole("heading", { name: /audit/i })).toBeVisible();
});
