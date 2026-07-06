# Frontend Architecture

## Purpose

The CorpusLab web app separates route registration, feature orchestration, reusable UI, and transport code so product workflows can grow without turning route files or the low-level HTTP client into catch-all modules.

## Ownership

`apps/web/src/pages` is the route-facing compatibility layer. Domain routes that already have feature ownership use one-line wrappers or re-exports; the remaining legacy implementations should move behind the same boundary during focused refactors. New product implementation belongs under `apps/web/src/features`.

Workbench domains use this structure when their complexity requires it:

```text
apps/web/src/features/workbench/<domain>/
  <Domain>Page.tsx              Route-level composition
  <Domain>Page.module.css       Route-specific layout
  components/
    *.tsx                       Domain presentation and controls
  hooks/
    use*.ts                     Server state, mutations, and workflow actions
  state/
    *.ts                        Reducers or state machines when needed
  utils/
    *.ts                        Pure calculations and formatting
  __tests__/
    *.test.tsx                  Focused feature tests
```

Small domains do not need every folder. Add a boundary when it removes real complexity rather than creating empty structure.

The public landing route is lazy-loaded and owned by
`features/marketing/landing`. Its command-center demonstration uses typed,
static scenarios and never calls workbench APIs. Motion orchestration stays in
focused hooks and configuration modules; section and component styling remains
in CSS modules. The simulation exposes the same product concepts as the
workbench without representing fixture state as live customer data.

The post-hero landing narrative is a second, deliberately separate boundary.
`LandingPage` only orders focused sections; typed `landingStoryData` fixtures
connect deterministic diagnosis, Eval Lab comparison, CI gating, audit reports,
and the local-first privacy boundary. Only the diagnosis stage is interactive.
Later sections are static product artifacts so they explain the workflow without
imitating live API state or introducing marketing-only product logic.

The hero's SVG-first Evidence Reactor is a decorative projection of the existing
`CommandCenterScenario`; it does not own another simulation state. Documents,
candidate and supported evidence, answerability coverage, gate state, and report
readiness are derived from that scenario and rendered with transform, opacity,
and path-progress motion. The reactor is pointer-transparent, hidden from the
accessibility tree, static under reduced motion, and introduces no canvas,
WebGL, image, API, or dependency boundary.

Landing sections declare their visual tone through semantic data attributes.
The marketing header uses a route-scoped, animation-frame-batched hook to read
the surface beneath its fixed probe line and select hero, dark-glass, or
light-glass presentation. Other public routes retain the static marketing
header and do not install landing scroll behavior.

## Page And Component Boundaries

- Page components compose hooks and panels; they should not contain every control and data transformation for a workflow.
- Move a component when it has a clear domain responsibility or can be tested independently.
- Move state transitions into a hook or reducer when a page coordinates several loading states, mutations, or dependent selections.
- Treat roughly 250 lines as a review signal, not an automatic rule. Split by ownership, not arbitrary line counts.
- Put cross-domain controls in `apps/web/src/components`. Keep domain-specific controls inside their feature folder.
- Nested presentation components should receive typed data and callbacks instead of importing API functions directly.

Current workbench implementations live in:

- `features/workbench/sources`: corpus upload, document library, and chunk inspection.
- `features/workbench/retrieval`: a thin retrieval page, `useRetrievalWorkbench` orchestration, focused query/filter/embedding panels, pure filter utilities, evidence results, and debugger handoff.
- `features/workbench/traces`: a focused run list, trace query/tab orchestration hook, and separate summary, failure, evidence, metrics, timeline, rerun, and Quality components.
- `features/workbench/eval-lab`: datasets, cases, experiments, gates, and CI quality runs.
- `features/workbench/home`: persisted six-step guided demo, sample-source mutations, workspace health, and next actions; `pages/OverviewPage.tsx` remains a thin compatibility export.
- `features/workbench/reports`: report creation, generated report lists, focused report detail, privacy status, and Markdown copy actions.
- `features/workbench/settings`: workspace, API-key, runtime, and privacy settings; the route file remains a thin compatibility export.

## Workbench Shell And Information Architecture

The authenticated shell is deliberately separate from marketing composition. A
single typed manifest under `features/workbench/shell` owns navigation labels,
workflow order, active-route matching, help copy, and breadcrumbs. Do not copy
route labels into the layout or infer parent navigation from string fragments in
page components.

The canonical hierarchy is:

```text
Setup:   Home → Corpus
Debug:   Retrieval → Trace Debugger
Quality: Eval Lab → CI Runs
Share:   Audit Reports
Admin:   Settings
```

Existing `/app` URLs remain stable. CI Runs is a focused frontend view at
`/app/evals?view=ci-runs`; `/app/evals` remains the default Eval Lab view.
Detail routes activate their parent area and expose linked breadcrumbs without
requiring extra API calls.

`WorkbenchPageHeader` provides the shared title, explanation, action, metadata,
and back-link structure. `WorkbenchEmptyState` provides explicit primary and
secondary next actions. Pages keep workflow-specific controls and data fetching
inside their feature domains. Legacy shared workbench classes are scoped under
`data-workbench-shell`, while all new shell, page, and component styling uses CSS
modules.

## API Boundary

`apps/web/src/lib/api/client.ts` owns only transport concerns:

- API base URL resolution.
- JSON request helpers.
- JSON response parsing.
- Structured HTTP errors.

`ApiError` preserves the HTTP status, optional backend code, and raw response body for diagnostics. User-facing code receives the structured backend `error.message`; non-JSON and empty failures use a stable `Request failed with <status>` fallback instead of exposing proxy or server output.

Product requests and request/response types belong to domain modules such as `sources.ts`, `retrieval.ts`, `traces.ts`, `evalLab.ts`, `auth.ts`, `overview.ts`, `config.ts`, and `embeddings.ts`.

UI tests should mock the narrow domain boundary or the HTTP route relevant to the workflow. Avoid broad global-fetch fixtures when a focused domain mock communicates intent better.

## Styling

- Global CSS is limited to tokens, base rules, utilities, and application-shell primitives.
- Route and component styling uses CSS modules.
- A feature module should not depend on selectors owned by another feature.
- Stable boards, score bars, tabs, and tool layouts need explicit responsive dimensions so dynamic data cannot shift controls or overlap text.
- Marketing motion must preserve visible labels and state, provide controls for automatically changing content, and render a complete static experience under `prefers-reduced-motion`.
- Landing story examples must remain typed, deterministic, and clearly framed as examples; they do not fetch workbench state.
- New UI must be checked at desktop, tablet, and mobile widths with no horizontal overflow.

## Testing

- Pure utilities receive unit tests without rendering React.
- Hooks with meaningful transitions receive focused hook or feature tests.
- Page tests verify loading, error, empty, success, and mutation states.
- Playwright covers workflows that cross routes or require the real API boundary.
- Visible regressions require screenshots or recordings in the PR.

Playwright specifications are separated by ownership: marketing behavior,
workbench feature workflows, real memory-backed flows, and workbench quality
gates. Shared authentication, exact endpoint mocks, and layout assertions live
under `tests/e2e/support`; deterministic hostile-data fixtures live under
`tests/e2e/fixtures`. Do not use a catch-all API mock that can hide a new route
dependency.

The workbench quality suite verifies every primary route at desktop, tablet,
and mobile widths, then exercises document, trace, experiment, report, and API
key surfaces with long technical strings. Review screenshots are opt-in and
remain under ignored Playwright output. The complete checklist is in
`docs/workbench-ui-quality.md`.

Before merging frontend work, run:

```sh
cd apps/web
npm run typecheck
npm run lint
npm test -- --run
npm run build
npm run format:check
```

Run Playwright when navigation, responsive layout, authentication, or a multi-step workflow changes.
