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

## Reliability

Workbench routes render inside an error boundary so a malformed response cannot replace the entire application with a blank screen. API timestamps serialize as RFC3339 strings. The core compatibility codec can still deserialize the legacy array representation already stored in Postgres JSON columns.

The frontend date utility also treats timestamp values as untrusted input and displays `Time unavailable` instead of throwing during rendering.
