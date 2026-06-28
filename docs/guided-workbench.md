# Guided Workbench

CorpusLab organizes the product around one operational sequence:

1. Add documents in **Corpus**.
2. Index evidence and ask a question in **Test retrieval**.
3. Save and diagnose the result in **Runs**.
4. Record expected evidence and run regression checks in **Quality**.
5. Share the diagnosis from **Reports**.

## Home

Home derives setup progress from the real overview API. It recommends the first incomplete step and collapses the checklist after documents, embeddings, retrieval, traces, and quality coverage are available. Core metrics remain visible; lower-level corpus totals and profile data live under System details.

## Navigation

The sidebar uses workflow groups instead of presenting every subsystem as an equal destination. The workspace header contains the current workspace, page breadcrumb, system health, help, and account controls. Page-specific actions remain on their page.

## Focused Routes

Each route has one primary job:

- `/app/sources` uploads files and lists the document library.
- `/app/sources/:documentId` inspects one document, its extraction findings, and chunks.
- `/app/retrieval` asks one question. Filters, indexing status, and `top_k` live under **Advanced**.
- `/app/traces` searches saved retrieval runs.
- `/app/traces/:traceId` diagnoses one run through **Summary**, **Evidence**, **Timeline**, and **Compare** tabs.
- `/app/evals` summarizes datasets, experiments, and CI gates.
- `/app/evals/datasets/:datasetId` manages expected-evidence cases and runs experiments.
- `/app/evals/experiments/:experimentId` leads with the gate outcome and failed cases before detailed metrics.
- `/app/reports` prioritizes failed CI gates, weak runs, and corpus findings that are ready for review.
- `/app/settings` separates Workspace, API keys, Runtime, and Privacy concerns.

Existing top-level redirects remain available, so older bookmarks continue to reach the corresponding workbench page.

## Interaction Standard

Default views contain the controls required for the normal workflow. Expert controls are disclosed only when requested. Corpus hides chunking parameters under **Advanced chunking**; Test retrieval hides filters, indexing controls, and result count under **Advanced**. Saving a retrieval result opens its debugger directly. Adding a run to Quality requires an explicit dataset and expected-evidence selection.

Workbench route styles use scoped CSS modules. `features/workbench/workbench.css` contains only shared primitives such as panels, buttons, badges, and form grids. Route containers, components, hooks, and domain clients remain separate.

## Reliability

Workbench routes render inside an error boundary so a malformed response cannot replace the entire application with a blank screen. API timestamps serialize as RFC3339 strings. The core compatibility codec can still deserialize the legacy array representation already stored in Postgres JSON columns.

The frontend date utility also treats timestamp values as untrusted input and displays `Time unavailable` instead of throwing during rendering.

## Browser Verification

The Playwright suite starts a memory-backed Rust API on port `18080` and Vite on port `5173`. Its real workflow logs in, uploads a document, indexes embeddings, tests retrieval, saves and opens a run, compares retrieval modes, and records expected evidence in Quality. Separate checks cover desktop, tablet, and mobile widths and reject horizontal overflow.
