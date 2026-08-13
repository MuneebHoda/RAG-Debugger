# Design-Partner Feedback

Use this template for a structured CorpusLab design-partner session. Keep answers sanitized and store them only in an approved private location.

> Do not paste proprietary documents, prompts, generated answers, credentials, cookies, customer data, raw trace content, full reports, private source paths, or other sensitive material into a public GitHub issue. Use synthetic reproductions, opaque IDs, aggregate metrics, stable failure codes, and redacted screenshots. Report security vulnerabilities privately through [`SECURITY.md`](../SECURITY.md).

## Session Context

- Date:
- Participants and roles:
- CorpusLab commit or version:
- Operating system and environment:
- Storage backend used:
- Synthetic or approved-local test data:

## RAG Stack And Deployment

- Application/frameworks:
- Retrieval library or service:
- Embedding model/provider:
- Vector or search storage:
- Reranker or generation provider:
- Deployment model (local, private cloud, hosted, hybrid):
- Existing tracing or evaluation tools:

## Corpus And Retrieval Architecture

- Document types and approximate corpus scale:
- Extraction/OCR path:
- Chunking strategy:
- Metadata filters:
- Retrieval modes and `top_k`:
- Reranking/citation behavior:
- Index refresh process:

## Current Failure And Debugging Workflow

- Most frequent retrieval failures:
- Highest-impact failure:
- How failures are discovered today:
- How evidence is inspected today:
- How fixes are compared today:
- Current regression or release gate:
- Typical time from report to root cause:

## Onboarding Results

- Installation completed? If not, where did it stop?
- Time to first login:
- Time to first useful saved or imported trace:
- Commands or prerequisites that caused friction:
- Errors encountered, with sanitized status/error codes:
- Was the local-first boundary clear?
- Was project/API-key setup clear?
- Was the privacy-mode choice clear?

## Workflow Usefulness

Rate each item from 1 (not useful) to 5 (essential), then explain.

- Ranked evidence and score explanation:
- Answerability status:
- Failure labels and primary diagnosis:
- Trace comparison:
- Expected-evidence editor:
- Eval Lab experiment/regression view:
- CI gate:
- Audit report:
- Markdown export:
- Native JSON ingestion:
- OTLP ingestion:

## Privacy And Data Residency

- What data may never leave the partner environment?
- Are bounded snippets permitted? Under what approval?
- Are metadata-only traces sufficient for any workflows?
- Required retention/deletion behavior:
- Required access controls or audit evidence:
- Is a self-hosted runner acceptable for CI?
- What would be required before hosted use?

## Missing Integrations

- Framework or SDK integrations required:
- Trace/telemetry format required:
- Retrieval/vector systems required:
- CI/CD systems required:
- Identity/access systems required:
- Export/report destinations required:

## Repeated Use And Payment

- Features required for weekly repeated use:
- Features required for team adoption:
- Features required before payment:
- Preferred local/private/hosted operating model:
- Who would own rollout and budget?
- Willingness to participate in another session:
- Preferred next validation scenario:

## Defects

Create one entry per defect.

- Sanitized title:
- Severity: blocker / high / medium / low
- Reproducibility: always / intermittent / once
- Environment and CorpusLab commit:
- Synthetic setup:
- Sanitized reproduction steps:
- Expected behavior:
- Actual behavior:
- Safe error/status/failure-label evidence:
- Private security report required? yes / no

## Session Summary

- Strongest product signal:
- Largest adoption blocker:
- Most important missing capability:
- Privacy concern requiring follow-up:
- Recommended issue or experiment:
- Follow-up owner and date:
