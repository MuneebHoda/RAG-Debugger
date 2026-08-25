# Design-Partner Onboarding

CorpusLab is a RAG debugging, evaluation, and audit platform for teams building document-based AI systems. This design-partner package runs locally in your environment. Hosted access, public signup, billing, and production infrastructure are not available yet.

The current engagement is controlled product validation with a small number of RAG builders. Sensitive data does not need to leave the machine running CorpusLab.

## Who This Is For

The best current partner is a developer or team that:

- owns a document-based AI or retrieval-augmented generation system;
- can describe recurring retrieval failures and expected evidence;
- can run a Rust, Node.js, Docker, and Postgres development stack locally;
- is willing to test with synthetic or approved local data first; and
- can provide sanitized workflow feedback without sharing customer content publicly.

CorpusLab is useful when a plausible answer may be grounded in the wrong chunk, weak evidence, stale embeddings, duplicate evidence, or a retrieval configuration that regressed.

It is not a chatbot builder, vector database, framework replacement, generic APM product, or hosted SaaS.

## What You Can Validate Today

The complete loop is:

> Observe a real RAG failure -> inspect its retrieval evidence -> diagnose it -> preserve it as an evaluation -> compare fixes -> prevent regressions through CI.

Locally, you can:

- ingest synthetic or approved text, Markdown, HTML, and embedded-text PDF documents;
- build local embeddings and compare lexical, vector, and hybrid retrieval;
- inspect ranked evidence, score signals, citations, answerability, and failure labels;
- save and compare retrieval traces;
- import external traces through native JSON or OTLP/HTTP protobuf;
- build expected-evidence datasets and compare Eval Lab experiments;
- run a CI gate against a reachable local CorpusLab instance; and
- create privacy-classified audit reports with controlled Markdown export.

## Prerequisites

- Rust stable through `rustup`, with Rustfmt and Clippy.
- Node.js 24 or newer.
- Docker Desktop or another Docker daemon.
- OpenSSL.
- [`just`](https://github.com/casey/just).
- Git and a modern browser.
- Python 3 only if you want to run the OTLP SDK example.
- A standard OpenTelemetry Collector binary or container only if you want the Collector path.

There is no published minimum hardware profile yet. The current embedding baseline is local and CPU-based; corpus size and Docker resources affect indexing time.

## Install And Start

```sh
git clone https://github.com/MuneebHoda/RAG-Debugger.git
cd RAG-Debugger
cp .env.example .env
npm --prefix apps/web ci
openssl rand -base64 32
```

Place the generated password after `RAG_DEBUGGER_BOOTSTRAP_PASSWORD=` in `.env`. Keep or replace the local bootstrap email. The file is ignored by Git; never commit it or reuse the generated password elsewhere.

Start Postgres and run migrations:

```sh
just db-up
just db-migrate
```

Start the API:

```sh
just api
```

Start the web app in a second terminal:

```sh
just web
```

Open `http://127.0.0.1:5173/login`. The API is at `http://127.0.0.1:8080`; liveness and readiness are at `/healthz` and `/readyz`.

## Fastest Useful Demo

After login:

1. On **Home**, choose **Load sample corpus**.
2. Choose **Index sample**.
3. Choose **Test recommended query**.
4. On **Retrieval**, choose **Run retrieval** and inspect **Evidence Summary**.
5. Choose **Debug this run**.
6. On the trace, choose **Create audit report**, keep `metadata_only`, and choose **Create report**.

This path is covered by the repository's real memory-backed Playwright workflow and normally produces the first useful trace in well under 15 minutes after prerequisites are installed. See [Guided Demo](guided-demo.md) and the facilitator [Demo Script](demo-script.md).

## Projects And Workspace Scope

Local startup creates a workspace-owned default project. The guided demo creates a separate deterministic project named `CorpusLab Guided Demo`. The current workbench does not provide general project creation or switching.

For external trace ingestion, use the **Trace ingestion project** shown under **Settings -> API keys**. The API also exposes the same default through authenticated `GET /api/v1/projects/current`. Do not substitute a project ID from another workspace; inaccessible and nonexistent IDs receive the same sanitized response.

## Create A Scoped Ingestion Key

1. Open **Settings -> API keys**.
2. Confirm the displayed current project and its privacy policy.
3. In the create-key form, select **Trace ingestion**.
4. Create the key and copy the one-time `clab_...` secret.
5. Store the secret in a local environment variable or secret manager. CorpusLab stores only its hash and cannot show the secret again.
6. Revoke the key from the same page after the session if it is no longer needed.

Never put the key in a committed file, screenshot, issue, recording, shell script, or command output.

## Native JSON Ingestion

Use the repository's versioned example rather than creating a parallel contract:

```bash
export CORPUSLAB_API_URL=http://127.0.0.1:8080
export CORPUSLAB_PROJECT_ID='the project UUID shown in Settings'
read -r -s CORPUSLAB_API_KEY
export CORPUSLAB_API_KEY
printf '\n'
./scripts/ingest-trace-example.sh
```

The script sends deterministic, synthetic `snippets_allowed` data. It uses a protected temporary header file so the API key is not passed as a command-line argument. Repeating the request updates `native-demo-001` instead of creating another trace.

For the complete schema, bounds, retry semantics, and stable errors, use [Local Trace Ingestion](trace-ingestion.md).

## Direct OTLP/HTTP Export

CorpusLab accepts uncompressed OTLP/HTTP protobuf at `/api/v1/otel/v1/traces`. Set the standard exporter variables:

```sh
export OTEL_EXPORTER_OTLP_TRACES_ENDPOINT="$CORPUSLAB_API_URL/api/v1/otel/v1/traces"
export OTEL_EXPORTER_OTLP_TRACES_PROTOCOL=http/protobuf
export OTEL_EXPORTER_OTLP_TRACES_HEADERS="Authorization=Bearer%20$CORPUSLAB_API_KEY,x-corpuslab-project-id=$CORPUSLAB_PROJECT_ID"
export OTEL_EXPORTER_OTLP_COMPRESSION=none
```

Run the deterministic Python example without an external model:

```sh
python3 -m venv .venv
. .venv/bin/activate
pip install -r examples/trace-ingestion/python/requirements.txt
python examples/trace-ingestion/python/basic_ingest.py
```

OTLP is always stored as `metadata_only`; telemetry attributes cannot choose a workspace, project, or weaker privacy policy.

## Collector-Based OTLP/HTTP

The checked-in Collector configuration is [`examples/trace-ingestion/otel-collector-config.yaml`](../examples/trace-ingestion/otel-collector-config.yaml). Keep the three `CORPUSLAB_*` variables set, run OpenTelemetry Collector 0.158.0 with that file, and direct your SDK to `http://127.0.0.1:4318/v1/traces` using HTTP protobuf without compression.

The Collector forwards to `$CORPUSLAB_API_URL/api/v1/otel/v1/traces` with the scoped key and project header. Review the [Trace Ingestion](trace-ingestion.md) guide before adapting semantic attributes from a real application.

## Verify The Imported Trace

1. Open **Trace Debugger** at `/app/traces`.
2. Select the newest imported trace.
3. Confirm source (`native` or `otlp/http`), mapping status, privacy mode, external identity, safe configuration, evidence, limitations, and span hierarchy.
4. Verify that restricted content is absent for the selected mode.

Imported traces cannot be rerun in CorpusLab. The originating application owns execution; CorpusLab stores the observational result.

## Evals, Comparisons, Reports, And CI

- A normal Retrieval or Trace result can be saved with **Add to Quality** after selecting exact-chunk or document-level expected evidence.
- A `full_local_only` native import can create a provenance-locked local Eval case after an exact query match and authorized evidence selection.
- `metadata_only` and `snippets_allowed` imports cannot create reproducible imported Eval cases because they retain no query.
- Any dataset containing full-local imported provenance is rejected from CI.
- Imported metadata traces may create metadata reports; snippet imports may create metadata or approved-snippet reports.
- Full-local imported traces and their experiments cannot create reports or enter Markdown, copy, download, or export paths.

For ordinary local cases, run at least two compatible Eval Lab experiments to review regression changes. Use [CI Eval Workflows](ci-eval-workflows.md) with a workspace-scoped `ci_eval_runs` key when a self-hosted or other runner can reach this CorpusLab API.

## Choosing A Privacy Mode

Project privacy is the upper bound. The requested native import mode cannot weaken it.

| Mode               | Use when                                                                        | Key restriction                                            |
| ------------------ | ------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `metadata_only`    | Structural timing, rank, score, citation, and status are enough                 | No query or snippets; no reproducible imported Eval case   |
| `snippets_allowed` | A bounded, explicitly approved evidence excerpt is needed                       | No query/prompt/answer; no reproducible imported Eval case |
| `full_local_only`  | A local investigation needs bounded query, prompt, answer, labels, and snippets | No CI, report, Markdown, clipboard, download, or export    |

Start with `metadata_only`. Use stronger modes only after confirming that every retained field is approved to remain in the local CorpusLab database. OTLP cannot opt into a stronger mode in v1.

Read [Privacy and Security](privacy-security.md), [Logging and Redaction](logging-redaction.md), and the [Privacy Review Checklist](privacy-review-checklist.md) before using real data.

## What Not To Submit

Do not put any of the following into public GitHub issues, pull requests, recordings, screenshots, or shared feedback:

- proprietary documents, raw chunks, customer prompts, generated answers, or trace bodies;
- personal, health, financial, legal, credential, or regulated data;
- API keys, passwords, cookies, session values, database URLs, headers, or secret hashes;
- customer names, private source paths, internal hostnames, or unreleased model/configuration names; or
- full audit-report bodies or Eval queries derived from production traffic.

Use synthetic reproductions, opaque IDs, aggregate counts, stable failure codes, and redacted screenshots.

## Reset Or Remove Local Test Data

- Revoke temporary API keys in **Settings -> API keys**.
- Delete individual Eval Lab cases from their dataset when they are no longer needed.
- CorpusLab does not yet expose selective source, trace, experiment, or report deletion in the workbench.
- For a disposable test environment only, stop the stack and delete the entire local Postgres volume:

```sh
docker compose down -v
```

This command permanently deletes all local CorpusLab database data. Confirm that the volume contains no data you need before running it. Restart with `just db-up`, `just db-migrate`, and `just api`.

## Troubleshooting

- **Docker unavailable:** start Docker Desktop or the Docker daemon, then rerun `just db-up`.
- **Database connection or schema error:** verify `DATABASE_URL` in `.env` and run `just db-migrate`.
- **Port occupied:** stop the older process using `8080` or `5173` before restarting.
- **Login fails:** verify the bootstrap email and non-empty password in `.env`, then restart the API.
- **Trace key rejected:** create a non-revoked **Trace ingestion** key and use the complete one-time value.
- **Project not found:** copy the project ID shown in the same workspace as the key.
- **OTLP rejected:** use `application/x-protobuf`, HTTP/protobuf, and no compression; OTLP JSON, gzip, and gRPC are unsupported.
- **Partially mapped:** inspect the trace's stable limitation codes and compare your attributes with [Semantic Mapping](trace-ingestion.md#semantic-mapping).
- **CI runner cannot connect:** a cloud-hosted runner cannot reach `127.0.0.1` on your development machine. Use a self-hosted runner or another explicitly reachable private address.

## Provide Sanitized Feedback

Use [Design-Partner Feedback](design-partner-feedback.md) during or after the session. Prefer reproducible steps against the guided corpus. For defects involving private data, replace content with synthetic equivalents and report only safe IDs, versions, status codes, and failure labels.

Security vulnerabilities must use the private process in [`SECURITY.md`](../SECURITY.md), not a public issue.

## Capture Screenshots Safely

Use the checked-in guided corpus or another synthetic fixture in a disposable workspace. Before capture:

1. Revoke or move past any one-time API-key secret display.
2. Confirm the page contains no private query, path, source name, report body, email address, internal hostname, browser notification, extension UI, or unrelated desktop content.
3. Use a fixed browser viewport, reduced motion, and the real current workbench UI.
4. Capture only the product region needed to explain the workflow.
5. Inspect every visible string at full resolution before sharing.
6. Add descriptive alt text that states the page and diagnostic state rather than making a marketing claim.

`npm run screenshots:workbench` creates deterministic QA captures under ignored Playwright output. Those fixtures are for layout review and may contain deliberately hostile long strings; do not publish or commit them as product evidence without inspecting every pixel. Never use a fabricated or AI-generated interface image as proof of implemented behavior.

## Current Limitations And Hosted Status

Review [Known Limitations](known-limitations.md) before beginning a real-data pilot. Hosted/private-alpha access is deferred to issue #29. There is no hosted onboarding, billing, public signup, deployment service, or production support promise in this package.

The immediate goal is to validate whether the local debugging, evaluation, CI, and audit loop solves recurring partner problems. Evidence from 3 to 5 design partners will guide hosted and deployment work.
