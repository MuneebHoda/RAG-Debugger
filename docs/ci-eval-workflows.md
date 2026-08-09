# CI Eval Workflows

CI Eval Workflows let teams run CorpusLab Eval Lab gates from automation. A GitHub Actions job can call CorpusLab with a dataset, branch, commit SHA, retrieval modes, and `fail_on_gate=true`. If the gate fails, the API returns a non-2xx status so the CI job can block a merge.

## Why This Matters

RAG systems regress quietly. A chunking change, embedding refresh, scoring tweak, or corpus update can make important evidence disappear. CI gates turn Eval Lab datasets into release checks:

- Run expected-evidence questions on every branch.
- Compare lexical, vector, and hybrid behavior.
- Record recall, precision, MRR, citation coverage, and latency.
- Detect newly failing cases against the latest run for the same dataset/config label.
- Preserve an export-ready report for engineering review.

## Setup

1. Start the API and web app.
2. Sign in to `/app/settings`.
3. Open the `API keys` tab and create a key for GitHub Actions.
4. Copy the one-time `clab_...` secret.
5. In the target repository, open **Settings → Secrets and variables → Actions** and store it as the `CORPUSLAB_API_KEY` repository secret.
6. Add the Eval Lab dataset UUID as the `CORPUSLAB_DATASET_ID` repository variable. Optionally add `CORPUSLAB_API_URL` and `CORPUSLAB_CONFIG_LABEL` variables.
7. Copy `docs/examples/github-actions-corpuslab-evals.yml` into `.github/workflows/corpuslab-evals.yml`.

The Actions runner must be able to reach the CorpusLab API. Set `CORPUSLAB_API_URL` to that reachable base URL. If CorpusLab is only available on a private local network, use a self-hosted runner and set the variable to the address reachable from that runner.

## Run Request

```http
POST /api/v1/eval-lab/ci/runs
Authorization: Bearer clab_...
Content-Type: application/json
```

Request body:

```json
{
  "dataset_id": "018f7a2a-6e2e-7000-a000-000000000001",
  "name": "Pull request retrieval gate",
  "branch": "feature/retrieval-change",
  "commit_sha": "abc123",
  "base_ref": "main",
  "head_ref": "feature/retrieval-change",
  "top_k": 5,
  "modes": ["hybrid", "vector", "lexical"],
  "config_label": "default",
  "fail_on_gate": true
}
```

If `modes` is empty, CorpusLab runs `hybrid`, `vector`, and `lexical`. If `top_k` is omitted or `0`, the API uses the configured retrieval default.

## Response Behavior

- `201 Created`: run saved and either the gate passed or `fail_on_gate=false`.
- `422 Unprocessable Entity`: run saved, but the gate failed and `fail_on_gate=true`.
- `401 Unauthorized`: missing, invalid, or revoked API key.
- `403 Forbidden`: API key does not include the `ci_eval_runs` scope.

The response includes:

- linked Eval Lab experiment
- gate status
- branch and commit metadata
- complete Eval Lab v2 regression details, including newly failed and recovered cases, versus the latest matching dataset/config run
- report JSON

The example writes only gate status, aggregate recall, failed-case count, opaque run/experiment IDs, and the configured label to logs and the Actions job summary. It deliberately never prints the response body because it may contain case queries and report content.

## Workbench Views

CI run history appears in:

- `/app/evals?view=ci-runs` under CI Runs.
- `/app/evals/ci-runs/:runId` as a focused detail view with revision metadata, thresholds, failed metrics, newly failing and recovered cases, failure labels, per-mode metrics, and report creation.
- Mission Control as latest gate status and recommended action.
- `/app/reports` as failed-gate report candidates with native CI audit-report creation.

Creating a report from a failed CI gate preserves the CI run source, branch, commit, configuration label, regression deltas, newly failed cases, and gate outcome. The action defaults to `metadata_only` and opens the generated report directly after creation.

## Gate Rule

The default Eval Lab gate passes when:

- average recall@k is at least `0.80`
- no critical missing-embedding failures exist
- weak-evidence cases are no more than 20% of the dataset

The gate is deterministic and local. There is no hosted LLM judge in this pass.

## Improving A Failing Gate

When a gate fails:

1. Open the CI run report.
2. Review newly failing cases first.
3. Check whether expected evidence is missing, weak, duplicated, heading-only, or in the wrong chunk.
4. Rerun the same query in Retrieval and Trace Debugger.
5. Fix extraction, chunking, embeddings, scoring weights, or expected-evidence cases.
6. Rerun CI.

## Example Workflow

See `docs/examples/github-actions-corpuslab-evals.yml`.
