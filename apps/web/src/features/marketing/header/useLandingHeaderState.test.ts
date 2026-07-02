import { describe, expect, it } from "vitest";

import { resolveLandingHeaderState } from "./useLandingHeaderState";

describe("resolveLandingHeaderState", () => {
  it("keeps the transparent hero state only at the top of the page", () => {
    expect(resolveLandingHeaderState([section("hero", 0, 900)], 0)).toBe(
      "hero",
    );
    expect(resolveLandingHeaderState([section("hero", -40, 860)], 40)).toBe(
      "dark",
    );
  });

  it("uses the tone of the section beneath the header probe", () => {
    const sections = [
      section("dark", -900, 40),
      section("light", 40, 840),
      section("dark", 840, 1240),
    ];

    expect(resolveLandingHeaderState(sections, 900)).toBe("light");
  });

  it("returns to dark glass over a later dark section", () => {
    const sections = [section("light", -600, 40), section("dark", 40, 500)];

    expect(resolveLandingHeaderState(sections, 1600)).toBe("dark");
  });
});

function section(tone: "hero" | "dark" | "light", top: number, bottom: number) {
  const element = document.createElement("section");
  element.dataset.landingHeaderTone = tone;
  element.getBoundingClientRect = () => ({ top, bottom }) as DOMRect;
  return element;
}
