import { expect, test } from "@playwright/test";

import {
  stressApiKey,
  stressChunk,
  stressExperiment,
  stressReport,
  stressSource,
  stressTrace,
  stressTraceSummary,
  stressValues,
} from "./fixtures/workbenchStress";
import {
  expectElementsNotToOverlap,
  expectNoHorizontalOverflow,
  expectSinglePageHeading,
  expectTextContained,
} from "./support/layoutAssertions";
import { installWorkbenchMocks } from "./support/workbenchMocks";

const viewports = [
  { name: "desktop", width: 1440, height: 1000 },
  { name: "tablet", width: 768, height: 900 },
  { name: "mobile", width: 390, height: 900 },
] as const;

const routeCases = [
  {
    path: "/app",
    heading: "Home",
    nav: "Home",
    primary: { role: "button", name: "Load sample corpus" },
  },
  {
    path: "/app/sources",
    heading: "Corpus",
    nav: "Corpus",
    primary: { role: "button", name: "Ingest files" },
  },
  {
    path: "/app/retrieval",
    heading: "Test retrieval",
    nav: "Test retrieval",
    primary: { role: "button", name: "Run retrieval" },
  },
  {
    path: "/app/traces",
    heading: "Runs",
    nav: "Runs",
    primary: { role: "link", name: "New retrieval test" },
  },
  {
    path: "/app/evals",
    heading: "Quality",
    nav: "Quality",
    primary: { role: "button", name: "New dataset" },
  },
  {
    path: "/app/reports",
    heading: "Audit reports",
    nav: "Reports",
    primary: { role: "link", name: "Continue the guided demo" },
  },
  {
    path: "/app/settings",
    heading: "Settings",
    nav: "Settings",
    primary: { role: "tab", name: "Workspace" },
  },
] as const;

for (const viewport of viewports) {
  test.describe(`${viewport.name} workbench routes`, () => {
    for (const routeCase of routeCases) {
      test(`${routeCase.heading} exposes a stable semantic layout`, async ({
        page,
      }) => {
        await installWorkbenchMocks(page);
        await page.setViewportSize(viewport);
        await page.goto(routeCase.path);

        await expectSinglePageHeading(page, routeCase.heading);
        const primary = page.getByRole(routeCase.primary.role, {
          name: routeCase.primary.name,
          exact: true,
        });
        await expect(primary.first()).toBeVisible();

        const navigation = page.getByRole("complementary", {
          name: "Workspace navigation",
        });
        if (viewport.width <= 900) {
          const openNavigation = page.getByRole("button", {
            name: "Open navigation",
          });
          await openNavigation.focus();
          await openNavigation.press("Enter");
          await expect(navigation).toBeVisible();
        }

        const activeLink = navigation.getByRole("link", {
          name: routeCase.nav,
          exact: true,
        });
        await expect(activeLink).toHaveAttribute("aria-current", "page");

        if (viewport.width <= 900) {
          await navigation
            .getByRole("button", { name: "Close navigation" })
            .click();
        }

        await expectElementsNotToOverlap(
          page.getByRole("heading", { level: 1 }),
          primary.first(),
        );
        await expectNoHorizontalOverflow(page);
      });
    }
  });
}

test("mobile navigation is keyboard-operable and reduced-motion safe", async ({
  page,
}) => {
  await installWorkbenchMocks(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 390, height: 900 });
  await page.goto("/app/traces");

  const openNavigation = page.getByRole("button", {
    name: "Open navigation",
  });
  await openNavigation.focus();
  await openNavigation.press("Enter");
  await expect(openNavigation).toHaveAttribute("aria-expanded", "true");

  const navigation = page.getByRole("complementary", {
    name: "Workspace navigation",
  });
  await expect(navigation).toBeVisible();
  expect(
    await navigation.evaluate(
      (element) => getComputedStyle(element).transitionDuration,
    ),
  ).toBe("0s");

  await page.keyboard.press("Escape");
  await expect(openNavigation).toHaveAttribute("aria-expanded", "false");
  await expect(openNavigation).toBeFocused();
  await expectNoHorizontalOverflow(page);
});

test("marketing and workbench shells remain route-isolated", async ({
  page,
}) => {
  await installWorkbenchMocks(page);

  await page.goto("/");
  await expect(
    page.getByRole("navigation", { name: "Public navigation" }),
  ).toBeVisible();
  await expect(
    page.getByRole("complementary", { name: "Workspace navigation" }),
  ).toHaveCount(0);

  await page.goto("/app");
  await expect(
    page.getByRole("complementary", { name: "Workspace navigation" }),
  ).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Public navigation" }),
  ).toHaveCount(0);
});

test.describe("hostile technical content", () => {
  test.beforeEach(async ({ page }) => {
    await installWorkbenchMocks(page, {
      sources: [stressSource],
      documentChunks: { [stressValues.document]: [stressChunk] },
      traces: [stressTraceSummary],
      traceDetails: { [stressValues.trace]: stressTrace },
      experiments: [stressExperiment],
      experimentDetails: { [stressValues.experiment]: stressExperiment },
      reports: [stressReport],
      reportDetails: { [stressValues.report]: stressReport },
      apiKeys: [stressApiKey],
    });
    await page.setViewportSize({ width: 390, height: 900 });
  });

  test("document paths and chunk text stay contained", async ({ page }) => {
    await page.goto(`/app/sources/${stressValues.document}`);
    const heading = page.getByRole("heading", {
      level: 1,
      name: stressValues.documentPath,
    });
    await expectTextContained(heading);
    await expectTextContained(
      page.getByText(stressValues.snippet, { exact: true }),
    );
    await expectNoHorizontalOverflow(page);
  });

  test("dense trace evidence and failure labels stay contained", async ({
    page,
  }) => {
    await page.goto(`/app/traces/${stressValues.trace}`);
    await expectTextContained(
      page.getByRole("heading", { level: 1, name: stressValues.query }),
    );
    await expect(
      page.getByRole("heading", {
        name: "Candidates do not directly support an answer",
      }),
    ).toBeVisible();
    await page.getByRole("tab", { name: "Evidence" }).click();
    await expectTextContained(
      page.getByText(stressValues.snippet, { exact: true }),
    );
    await expect(page.getByLabel("Score breakdown")).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });

  test("experiment failures and dense metrics stay contained", async ({
    page,
  }) => {
    await page.goto(`/app/evals/experiments/${stressValues.experiment}`);
    await expectTextContained(
      page.getByRole("heading", {
        level: 1,
        name: stressExperiment.name,
      }),
    );
    await expectTextContained(page.getByText(stressValues.query));
    await expect(
      page.getByRole("heading", { name: "Mode comparison" }),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });

  test("report titles and Markdown-sensitive metadata stay contained", async ({
    page,
  }) => {
    await page.goto(`/app/reports/${stressValues.report}`);
    await expectTextContained(
      page.getByRole("heading", {
        level: 1,
        name: stressValues.reportTitle,
      }),
    );
    await expect(page.getByText("metadata only")).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });

  test("long API-key names stay contained", async ({ page }) => {
    await page.goto("/app/settings?tab=api-keys");
    await expectTextContained(page.getByText(stressValues.apiKeyName));
    await expect(
      page.getByRole("button", { name: `Revoke ${stressValues.apiKeyName}` }),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });
});

test.describe("workbench review screenshots", () => {
  test.skip(
    process.env.CORPUSLAB_CAPTURE_WORKBENCH !== "1",
    "Run through npm run screenshots:workbench.",
  );

  test("captures workbench review screenshots", async ({ page }, testInfo) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await installWorkbenchMocks(page);

    for (const viewport of [
      { width: 1440, height: 1000 },
      { width: 390, height: 900 },
    ]) {
      await page.setViewportSize(viewport);
      for (const routeCase of routeCases) {
        await page.goto(routeCase.path);
        await expectSinglePageHeading(page, routeCase.heading);
        await expectNoHorizontalOverflow(page);
        await page.screenshot({
          animations: "disabled",
          path: testInfo.outputPath(
            `${routeSlug(routeCase.path)}-${viewport.width}x${viewport.height}.png`,
          ),
        });
      }
    }
  });
});

function routeSlug(path: string) {
  return path === "/app" ? "home" : path.replace("/app/", "");
}
