import { useEffect, useState } from "react";

export type LandingHeaderState = "hero" | "dark" | "light";

const HEADER_PROBE_OFFSET = 78;
const TOP_SCROLL_EPSILON = 4;
const TONE_SELECTOR = "[data-landing-header-tone]";

export function useLandingHeaderState(
  enabled: boolean,
  routeKey: string,
): LandingHeaderState {
  const [snapshot, setSnapshot] = useState({
    routeKey,
    state: "hero" as LandingHeaderState,
  });

  useEffect(() => {
    if (!enabled) return undefined;

    let frameId: number | null = null;
    const update = () => {
      frameId = null;
      const sections = document.querySelectorAll<HTMLElement>(TONE_SELECTOR);
      const nextState = resolveLandingHeaderState(
        sections,
        window.scrollY,
        HEADER_PROBE_OFFSET,
      );
      setSnapshot((current) =>
        current.routeKey === routeKey && current.state === nextState
          ? current
          : { routeKey, state: nextState },
      );
    };
    const scheduleUpdate = () => {
      if (frameId !== null) return;
      frameId = window.requestAnimationFrame(update);
    };

    update();
    window.addEventListener("scroll", scheduleUpdate, { passive: true });
    window.addEventListener("resize", scheduleUpdate, { passive: true });

    return () => {
      window.removeEventListener("scroll", scheduleUpdate);
      window.removeEventListener("resize", scheduleUpdate);
      if (frameId !== null) window.cancelAnimationFrame(frameId);
    };
  }, [enabled, routeKey]);

  if (!enabled) return "light";
  return snapshot.routeKey === routeKey ? snapshot.state : "hero";
}

export function resolveLandingHeaderState(
  sections: Iterable<HTMLElement>,
  scrollY: number,
  probeOffset = HEADER_PROBE_OFFSET,
): LandingHeaderState {
  if (scrollY <= TOP_SCROLL_EPSILON) return "hero";

  const measuredSections = Array.from(sections, (section) => ({
    bounds: section.getBoundingClientRect(),
    tone: readTone(section),
  }));

  let nearestTone: LandingHeaderState = "dark";
  for (const { bounds, tone } of measuredSections) {
    if (!tone) continue;

    if (bounds.top <= probeOffset) nearestTone = normalizeScrolledTone(tone);
    if (bounds.top <= probeOffset && bounds.bottom > probeOffset) {
      return normalizeScrolledTone(tone);
    }
  }

  return nearestTone;
}

function readTone(element: HTMLElement): LandingHeaderState | null {
  const tone = element.dataset.landingHeaderTone;
  return tone === "hero" || tone === "dark" || tone === "light" ? tone : null;
}

function normalizeScrolledTone(tone: LandingHeaderState) {
  return tone === "hero" ? "dark" : tone;
}
