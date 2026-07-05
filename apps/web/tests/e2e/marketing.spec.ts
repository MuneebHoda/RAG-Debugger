import { expect, test, type Page } from "@playwright/test";

import { authResponse } from "./support/auth";
import { expectNoHorizontalOverflow } from "./support/layoutAssertions";

test("renders the CorpusLab public site", async ({ page }) => {
  await page.goto("/");

  const landingHeader = page.locator("[data-landing-header-state]");
  await expect(landingHeader).toHaveAttribute(
    "data-landing-header-state",
    "hero",
  );
  await expect(
    page.getByRole("heading", { name: "See why your RAG answer failed." }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "Features" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Pricing" })).toBeVisible();
  await expect(
    page.getByLabel("Interactive RAG diagnosis simulation"),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Run the guided demo" }).first(),
  ).toHaveAttribute("href", "/app");
});

test("landing interactions remain accessible and layout-stable", async ({
  page,
}) => {
  await observeCumulativeLayoutShift(page);
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.goto("/");

  const landingHeader = page.locator("[data-landing-header-state]");
  const reactor = page.locator("[data-evidence-reactor]");

  await expect(reactor).toHaveAttribute("data-reactor-outcome", "failing");
  await expect(reactor).toHaveAttribute("data-reactor-gate", "failed");
  await expect(reactor).toHaveAttribute("data-reactor-coverage", "0");

  const strongScenario = page.getByRole("button", { name: "Strong" });
  await strongScenario.focus();
  await strongScenario.press("Enter");
  await expect(page.getByText("Direct evidence, release ready")).toBeVisible();
  await expect(page.getByText("Audit ready")).toBeVisible();
  await expect(reactor).toHaveAttribute("data-reactor-outcome", "strong");
  await expect(reactor).toHaveAttribute("data-reactor-gate", "passed");
  await expect(reactor).toHaveAttribute("data-reactor-coverage", "100");
  expect(
    await reactor.evaluate(
      (element) => getComputedStyle(element).pointerEvents,
    ),
  ).toBe("none");

  const unsupportedAnswer = page.getByRole("tab", {
    name: "Unsupported answer",
  });
  await unsupportedAnswer.focus();
  await unsupportedAnswer.press("ArrowLeft");
  await expect(
    page.getByRole("tab", { name: "Ranking drift" }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText("vector_lexical_disagreement")).toBeVisible();
  await expect(page.getByText("Supports answer")).toBeVisible();
  await expect(
    page.getByText(/compare lexical, vector, and hybrid runs/i),
  ).toBeVisible();

  await page
    .locator("#quality-loop")
    .evaluate((element) => element.scrollIntoView({ block: "start" }));
  await expect(landingHeader).toHaveAttribute(
    "data-landing-header-state",
    "light",
  );

  await page
    .locator("#ci-gate")
    .evaluate((element) => element.scrollIntoView({ block: "start" }));
  await expect(landingHeader).toHaveAttribute(
    "data-landing-header-state",
    "dark",
  );
  await expect(page.getByText("Merge blocked")).toBeVisible();

  await page
    .locator("#audit-report")
    .evaluate((element) => element.scrollIntoView({ block: "start" }));
  await expect(page.getByText("metadata_only")).toBeVisible();

  const finalCta = page.locator("#landing-cta");
  await expect(
    finalCta.getByRole("link", { name: "Run the guided demo" }),
  ).toHaveAttribute("href", "/app");
  await expect(
    finalCta.getByRole("link", { name: "View the debugger" }),
  ).toHaveAttribute("href", "/app/traces");
  await expectNoHorizontalOverflow(page);

  await page.waitForLoadState("networkidle");
  expect(await readCumulativeLayoutShift(page)).toBeLessThan(0.1);
});

test("mobile navigation and reduced-motion experience remain complete", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 390, height: 900 });
  await page.goto("/");

  await expect(page.locator("[data-evidence-reactor]")).toHaveAttribute(
    "data-reactor-motion",
    "static",
  );

  const menuButton = page.getByRole("button", { name: "Open menu" });
  await menuButton.click();
  await expect(
    page.getByRole("button", { name: "Close menu" }),
  ).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(menuButton).toHaveAttribute("aria-expanded", "false");
  await expect(menuButton).toBeFocused();

  await page.getByRole("tab", { name: "Missing source" }).click();
  await expect(
    page.getByText(/answer never entered the corpus/i),
  ).toBeVisible();
  await expect(page.getByText("missing_expected_evidence")).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: /turn a bad run into a regression test/i,
    }),
  ).toBeAttached();
  await expectNoHorizontalOverflow(page);
});

test("captures responsive landing screenshots", async ({ page }, testInfo) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  for (const viewport of [
    { width: 1440, height: 1100 },
    { width: 1280, height: 900 },
    { width: 1024, height: 900 },
    { width: 768, height: 900 },
    { width: 390, height: 900 },
  ]) {
    await page.setViewportSize(viewport);
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: "See why your RAG answer failed." }),
    ).toBeVisible();
    await expect(
      page.getByLabel("Interactive RAG diagnosis simulation"),
    ).toBeVisible();
    await revealLandingSections(page, viewport.height);
    await expectNoHorizontalOverflow(page);
    await page.screenshot({
      fullPage: true,
      path: testInfo.outputPath(
        `landing-${viewport.width}x${viewport.height}.png`,
      ),
    });
  }
});

test("renders pricing and auth pages", async ({ page }) => {
  await page.goto("/pricing");
  await expect(page.getByRole("heading", { name: "Team" })).toBeVisible();
  await expect(page.getByText("$299/mo")).toBeVisible();
  await expect(page.getByText(/platform units/i).first()).toBeVisible();

  await page.goto("/login");
  await page.route("**/api/v1/auth/me", (route) =>
    route.fulfill({ contentType: "application/json", json: authResponse }),
  );
  await page.route("**/api/v1/auth/login", (route) =>
    route.fulfill({ contentType: "application/json", json: authResponse }),
  );
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

async function revealLandingSections(page: Page, viewportHeight: number) {
  const pageHeight = await page.evaluate(
    () => document.documentElement.scrollHeight,
  );
  const step = Math.max(1, Math.floor(viewportHeight * 0.7));

  for (let offset = 0; offset < pageHeight; offset += step) {
    await page.evaluate(
      (scrollOffset) => window.scrollTo(0, scrollOffset),
      offset,
    );
    await page.waitForTimeout(50);
  }

  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(100);
}

async function observeCumulativeLayoutShift(page: Page) {
  await page.addInitScript(() => {
    const target = window as Window & { __corpusLabCls?: number };
    target.__corpusLabCls = 0;
    const observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        const shift = entry as PerformanceEntry & {
          hadRecentInput?: boolean;
          value?: number;
        };
        if (!shift.hadRecentInput) {
          target.__corpusLabCls =
            (target.__corpusLabCls ?? 0) + (shift.value ?? 0);
        }
      }
    });
    observer.observe({ buffered: true, type: "layout-shift" });
  });
}

async function readCumulativeLayoutShift(page: Page) {
  return page.evaluate(
    () => (window as Window & { __corpusLabCls?: number }).__corpusLabCls ?? 0,
  );
}
