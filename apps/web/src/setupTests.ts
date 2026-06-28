import "@testing-library/jest-dom/vitest";

declare global {
  var __setReducedMotionForTests: (matches: boolean) => void;
}

let prefersReducedMotion = false;
const reducedMotionListeners = new Set<(event: MediaQueryListEvent) => void>();

Object.defineProperty(globalThis, "matchMedia", {
  configurable: true,
  value: (query: string): MediaQueryList => {
    const isReducedMotionQuery = query.includes("prefers-reduced-motion");
    return {
      get matches() {
        return isReducedMotionQuery && prefersReducedMotion;
      },
      media: query,
      onchange: null,
      addEventListener: (
        _type: string,
        listener: EventListenerOrEventListenerObject | null,
      ) => {
        if (typeof listener === "function" && isReducedMotionQuery) {
          reducedMotionListeners.add(
            listener as (event: MediaQueryListEvent) => void,
          );
        }
      },
      removeEventListener: (
        _type: string,
        listener: EventListenerOrEventListenerObject | null,
      ) => {
        if (typeof listener === "function") {
          reducedMotionListeners.delete(
            listener as (event: MediaQueryListEvent) => void,
          );
        }
      },
      addListener: (listener) => {
        if (listener && isReducedMotionQuery) {
          reducedMotionListeners.add(listener);
        }
      },
      removeListener: (listener) => {
        if (listener) reducedMotionListeners.delete(listener);
      },
      dispatchEvent: () => true,
    } as MediaQueryList;
  },
});

globalThis.__setReducedMotionForTests = (matches: boolean) => {
  prefersReducedMotion = matches;
  const event = {
    matches,
    media: "(prefers-reduced-motion)",
  } as MediaQueryListEvent;
  reducedMotionListeners.forEach((listener) => listener(event));
};

class TestIntersectionObserver implements IntersectionObserver {
  readonly root = null;
  readonly rootMargin = "0px";
  readonly scrollMargin = "0px";
  readonly thresholds = [0];

  constructor(private readonly callback: IntersectionObserverCallback) {}

  disconnect() {}

  observe(target: Element) {
    const bounds = target.getBoundingClientRect();
    this.callback(
      [
        {
          boundingClientRect: bounds,
          intersectionRatio: 1,
          intersectionRect: bounds,
          isIntersecting: true,
          rootBounds: null,
          target,
          time: performance.now(),
        },
      ],
      this,
    );
  }

  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }

  unobserve() {}
}

class TestResizeObserver implements ResizeObserver {
  disconnect() {}
  observe() {}
  unobserve() {}
}

globalThis.IntersectionObserver = TestIntersectionObserver;
globalThis.ResizeObserver = TestResizeObserver;
