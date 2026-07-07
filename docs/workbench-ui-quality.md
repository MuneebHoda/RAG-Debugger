# Workbench UI Quality

## Purpose

Workbench UI changes must remain understandable and usable under real RAG data, not only short demonstration values. CorpusLab therefore treats navigation, responsive layout, technical-string containment, keyboard access, and route state as merge-time contracts.

This quality system is intentionally lightweight. It uses Vitest, Testing Library, and the existing Chromium Playwright project. It does not depend on an external visual-regression service, hosted credentials, or pixel-perfect snapshots.

## Automated Coverage

`apps/web/tests/e2e/workbench-quality.spec.ts` checks the authenticated Home, Corpus, Retrieval, Trace Debugger, Eval Lab, CI Runs, Audit Reports, and Settings routes at:

- Desktop: `1440x1000`
- Wide desktop: `1280x900`
- Compact desktop: `1024x900`
- Tablet: `768x900`
- Mobile: `390x900`

Every route check requires:

- One visible level-one heading.
- A visible primary action or useful empty-state action.
- The correct active workspace navigation item.
- A breadcrumb whose current item matches the page heading.
- No heading/action overlap.
- No document-level horizontal overflow.

Additional browser checks cover canonical workflow order, parent breadcrumbs on detail routes, focus entering the active mobile navigation item, Escape focus recovery, reduced-motion behavior, and isolation between the public marketing shell and authenticated workbench shell.

Nested surfaces are checked independently of document overflow. Empty-state
copy must retain a readable inline size, actions must remain inside their owning
panel, mobile tab labels must remain visible, and dense populated detail views
must contain their technical strings after a viewport resize.

Feature-level Vitest tests remain responsible for loading, error, empty, success, and mutation states. Playwright is reserved for layout, navigation, responsive behavior, and workflows that cross a real route or HTTP boundary.

## Visual System Primitives

Authenticated workbench pages share a small primitive set in
`apps/web/src/components/workbench`:

- `WorkbenchPageHeader`: one page title, explanation, metadata, actions, and back-link.
- `WorkbenchPanel`: section/card surfaces with optional heading, icon, description, and actions.
- `WorkbenchToolbar`: filter/action rows that wrap safely inside narrow panels.
- `WorkbenchStatusPill`: text-first status labels for evidence, gate, privacy, readiness, and count states.
- `WorkbenchMetricCard`: compact metric summaries for dashboard and quality totals.
- `WorkbenchEmptyState`: one explanation plus primary and secondary recovery actions.

Use these primitives for repeated workbench structure before adding a page-local
variant. Domain CSS modules still own workflow-specific grids, evidence cards,
forms, score bars, timelines, and intentionally scrollable technical previews.
Do not reintroduce global `.panel`, `.status-pill`, or feature-specific badge
systems for new workbench UI.

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

The screenshot command uses reduced motion and captures every major workbench
route at `1440`, `1280`, `1024`, `768`, and `390` pixel widths. It also captures
populated document, trace, experiment, report, and API-key detail surfaces at
compact-desktop and mobile widths. Output is written beneath Playwright's
ignored `test-results` directory. Attach useful captures to the PR; do not
commit them.

Before merge, run:

```sh
just check
just ci-check
git diff --check
```

## Assertion Design

Prefer semantic assertions such as headings, roles, accessible names, selected state, and visible recovery actions. Geometry checks should protect stable relationships such as a heading and primary action, a child inside its panel, or a minimum readable copy width. Avoid exact coordinates, animation timing internals, generated CSS class names, and pixel snapshots that fail on harmless rendering differences.

When a stress fixture exposes overflow, fix containment in the CSS module that owns the component. Use `min-width: 0`, responsive grid constraints, safe wrapping, or deliberate local scrolling. Do not hide document-level overflow without fixing the responsible child.
