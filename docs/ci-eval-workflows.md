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
3. Create a `CI API Keys` key.
4. Copy the one-time `clab_...` secret.
5. Store it in GitHub Actions as `CORPUSLAB_API_KEY`.
6. Copy `docs/examples/github-actions-corpuslab-evals.yml` into `.github/workflows/corpuslab-evals.yml`.

For hosted deployments, set `CORPUSLAB_API_URL` to the deployed API base URL.

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
- regression summary versus the latest matching dataset/config run
- report JSON

## Workbench Views

CI run history appears in:

- `/app/evals` under CI Gates.
- Mission Control as latest gate status and recommended action.
- `/app/reports` as failed-gate reports.

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
