# Workbench UI Quality

## Purpose

Workbench UI changes must remain understandable and usable under real RAG data, not only short demonstration values. CorpusLab therefore treats navigation, responsive layout, technical-string containment, keyboard access, and route state as merge-time contracts.

This quality system is intentionally lightweight. It uses Vitest, Testing Library, and the existing Chromium Playwright project. It does not depend on an external visual-regression service, hosted credentials, or pixel-perfect snapshots.

## Automated Coverage

`apps/web/tests/e2e/workbench-quality.spec.ts` checks the authenticated Home, Corpus, Test Retrieval, Runs, Quality, Audit Reports, and Settings routes at:

- Desktop: `1440x1000`
- Tablet: `768x900`
- Mobile: `390x900`

Every route check requires:

- One visible level-one heading.
- A visible primary action or useful empty-state action.
- The correct active workspace navigation item.
- No heading/action overlap.
- No document-level horizontal overflow.

Additional browser checks cover keyboard-operated mobile navigation, Escape focus recovery, reduced-motion behavior, and isolation between the public marketing shell and authenticated workbench shell.

Feature-level Vitest tests remain responsible for loading, error, empty, success, and mutation states. Playwright is reserved for layout, navigation, responsive behavior, and workflows that cross a real route or HTTP boundary.

## Deterministic Stress Fixtures

The fixtures under `apps/web/tests/e2e/fixtures` intentionally include:

- Deep and unbroken document paths.
- Long source, document, chunk, trace, experiment, report, and API-key identifiers.
- Long queries and snippets without natural wrapping points.
- Dense score breakdowns and many failure labels.
- Long report titles containing Markdown punctuation.

These are synthetic values and must never contain customer documents, production queries, credentials, or copied report bodies. Fixture objects use exported frontend API types so contract changes fail TypeScript validation.

Exact endpoint mocks live under `apps/web/tests/e2e/support`. Do not add a catch-all API response: an unmocked dependency should fail visibly so reviewers know that a route's data boundary changed.

## Workbench UI PR Checklist

For every visual or interaction change:

1. Check desktop, tablet, and mobile widths.
2. Confirm there is no horizontal overflow or incoherent overlap.
3. Test long technical strings, dense labels, and empty content.
4. Verify loading, error, empty, success, and mutation feedback at the lowest useful layer.
5. Navigate primary actions and menus with a keyboard.
6. Confirm visible focus, meaningful control names, semantic headings, and text labels for status.
7. Verify reduced-motion behavior when the changed surface animates.
8. Confirm marketing and workbench styles remain route-isolated.
9. Attach screenshots or a short recording to the PR.

Status, severity, gate outcome, and evidence strength must be expressed as text, not color alone. Tabs should use native button semantics with `tablist`, `tab`, and selected state where the interaction behaves like tabs.

## Commands

Run the automated workbench suite:

```sh
cd apps/web
npx playwright test tests/e2e/workbench-quality.spec.ts
```

Generate deterministic review captures:

```sh
cd apps/web
npm run screenshots:workbench
```

The screenshot command uses reduced motion and captures every major workbench route at desktop and mobile sizes. Output is written beneath Playwright's ignored `test-results` directory. Attach useful captures to the PR; do not commit them.

Before merge, run:

```sh
just check
just ci-check
git diff --check
```

## Assertion Design

Prefer semantic assertions such as headings, roles, accessible names, selected state, and visible recovery actions. Geometry checks should protect stable relationships such as a heading and primary action or viewport containment. Avoid exact coordinates, animation timing internals, generated CSS class names, and pixel snapshots that fail on harmless rendering differences.

When a stress fixture exposes overflow, fix containment in the CSS module that owns the component. Use `min-width: 0`, responsive grid constraints, safe wrapping, or deliberate local scrolling. Do not hide document-level overflow without fixing the responsible child.
