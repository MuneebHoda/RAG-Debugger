# Marketing Experience

## Purpose

CorpusLab's public site explains one product promise: every RAG answer should be traceable to defensible evidence. The landing page is an editorial product story rather than a collection of decorative cards.

## Route And Ownership

`apps/web/src/App.tsx` lazy-loads the landing route. Its implementation lives in `apps/web/src/features/marketing/landing`:

- `LandingPage.tsx`: thin section composer and Motion runtime boundary.
- `HeroSection.tsx`: full-bleed bitmap scene, primary promise, and first actions.
- `FailureStory.tsx`: keyboard-accessible RAG failure-stage diagnosis.
- `RetrievalDemo.tsx`: deterministic lexical, vector, and hybrid comparison tool.
- `EditorialSections.tsx`: outcome rail, capability narrative, enterprise trust, and closing CTA.
- `ProductTour.tsx`: stable product-screenshot tour.
- `landingData.ts`: typed display fixtures and image references.
- `motion.ts`: shared durations, easing, reveal variants, and viewport behavior.
- `useRovingTabs.ts`: shared arrow-key and focus behavior for tab sets.

Every visual section owns a CSS module. Global styles remain limited to tokens, base rules, utilities, and layout primitives.

## Interaction Model

The failure story and product tour use WAI-ARIA tab semantics, roving focus, and ArrowLeft/ArrowRight navigation. Query and retrieval-mode examples use native buttons with `aria-pressed`. All interactions work without API connectivity because they use typed, deterministic fixtures.

The mobile navigation exposes `aria-expanded`, closes on Escape, and closes after route selection. Visible focus states remain available across public navigation and demos.

## Motion And Performance

The landing page uses `motion` with `LazyMotion`, `domAnimation`, and `m` components. Animation is limited to opacity and transforms. Pointer movement updates Motion values and springs directly, avoiding React renders on each frame.

`prefers-reduced-motion` disables hero parallax, removes transition duration, and renders scroll-reveal sections immediately. The complete product story remains available in the static state.

The hero image uses `fetchPriority="high"`; below-fold images use lazy loading and stable aspect ratios. CI runs `npm run size:check` after the production build and rejects combined assets above:

- JavaScript: 180 KB gzip.
- CSS: 20 KB gzip.

## Visual Verification

Run all browser workflows:

```sh
cd apps/web
npx playwright test
```

Generate landing review artifacts:

```sh
cd apps/web
npm run screenshots:landing
```

The screenshot test scrolls through the page to activate intentional reveal states, returns to the top, and captures full-page images at 1440x1100, 1024x900, and 390x900. It also rejects horizontal overflow. Separate tests enforce CLS below 0.1 and verify the complete reduced-motion experience.

Product screenshots under `apps/web/public/product` remain real product-proof imagery. The hero and diagnostic scenes use themed bitmap assets from the same directory. Generated `apps/web/dist` output must never be edited directly.
