import { expect, type Locator, type Page } from "@playwright/test";

export async function expectNoHorizontalOverflow(page: Page) {
  const layout = await page.evaluate(() => {
    const viewport = document.documentElement.clientWidth;
    const offenders = [...document.querySelectorAll("body *")]
      .map((element) => {
        const bounds = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return { element, bounds, style };
      })
      .filter(
        ({ bounds, style }) =>
          bounds.width > 0 &&
          bounds.right > viewport + 1 &&
          style.display !== "none" &&
          style.visibility !== "hidden",
      )
      .slice(0, 8)
      .map(({ element, bounds }) => ({
        tag: element.tagName.toLowerCase(),
        className: element.className.toString(),
        right: Math.round(bounds.right),
        text: element.textContent?.trim().slice(0, 100) ?? "",
      }));

    return {
      content: document.documentElement.scrollWidth,
      viewport,
      offenders,
    };
  });
  expect(
    layout.content,
    `Horizontal overflow offenders: ${JSON.stringify(layout.offenders)}`,
  ).toBeLessThanOrEqual(layout.viewport);
}

export async function expectElementsNotToOverlap(
  first: Locator,
  second: Locator,
) {
  const firstBox = await first.boundingBox();
  const secondBox = await second.boundingBox();
  expect(firstBox).not.toBeNull();
  expect(secondBox).not.toBeNull();
  if (!firstBox || !secondBox) return;

  const overlaps = !(
    firstBox.x + firstBox.width <= secondBox.x ||
    secondBox.x + secondBox.width <= firstBox.x ||
    firstBox.y + firstBox.height <= secondBox.y ||
    secondBox.y + secondBox.height <= firstBox.y
  );
  expect(overlaps).toBeFalsy();
}

export async function expectElementContainedBy(
  element: Locator,
  container: Locator,
) {
  const elementBox = await element.boundingBox();
  const containerBox = await container.boundingBox();
  expect(elementBox).not.toBeNull();
  expect(containerBox).not.toBeNull();
  if (!elementBox || !containerBox) return;

  const tolerance = 1;
  expect(elementBox.x).toBeGreaterThanOrEqual(containerBox.x - tolerance);
  expect(elementBox.y).toBeGreaterThanOrEqual(containerBox.y - tolerance);
  expect(elementBox.x + elementBox.width).toBeLessThanOrEqual(
    containerBox.x + containerBox.width + tolerance,
  );
  expect(elementBox.y + elementBox.height).toBeLessThanOrEqual(
    containerBox.y + containerBox.height + tolerance,
  );
}

export async function expectMinimumInlineSize(
  locator: Locator,
  minimumWidth: number,
) {
  const box = await locator.boundingBox();
  expect(box).not.toBeNull();
  if (!box) return;
  expect(box.width).toBeGreaterThanOrEqual(minimumWidth);
}

export async function expectTextContained(locator: Locator) {
  await expect(locator).toBeVisible();
  const contained = await locator.evaluate((element) => {
    const range = document.createRange();
    range.selectNodeContents(element);
    const textBounds = range.getBoundingClientRect();
    const elementBounds = element.getBoundingClientRect();
    const tolerance = 1;

    return (
      textBounds.left >= elementBounds.left - tolerance &&
      textBounds.right <= elementBounds.right + tolerance &&
      textBounds.top >= elementBounds.top - tolerance &&
      textBounds.bottom <= elementBounds.bottom + tolerance
    );
  });
  expect(contained).toBeTruthy();
}

export async function expectSinglePageHeading(page: Page, name: string) {
  const headings = page.getByRole("heading", { level: 1 });
  await expect(headings).toHaveCount(1);
  await expect(headings).toHaveText(name);
}
