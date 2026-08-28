# CI Eval Workflows

CI Eval Workflows let teams run CorpusLab Eval Lab gates from automation. A GitHub Actions job can call CorpusLab with a dataset, branch, commit SHA, retrieval modes, and `fail_on_gate=true`. If the gate fails, the API returns a non-2xx status so the CI job can block a merge.

## Why This Matters

RAG systems regress quietly. A chunking change, embedding refresh, scoring tweak, or corpus update can make important evidence disappear. CI gates turn Eval Lab datasets into release checks:

- Run expected-evidence questions on every branch.
- Compare lexical, vector, and hybrid behavior.
- Record recall, precision, MRR, citation coverage, and latency.
- Detect newly failing cases only when the latest earlier run for the same dataset/config label has identical identity-defining provenance; otherwise record that no compatible baseline exists.
- Preserve an export-ready report for engineering review.

## Setup

1. Start the API and web app.
2. Sign in to `/app/settings`.
3. Open the `API keys` tab and create a key for GitHub Actions.
4. Copy the one-time `clab_...` secret.
5. In the target repository, open **Settings → Secrets and variables → Actions** and store it as the `CORPUSLAB_API_KEY` repository secret.
6. In the same **Actions** settings, add `CORPUSLAB_API_URL` as a required repository variable containing the CorpusLab API base URL reachable from the runner.
7. Add the Eval Lab dataset UUID as the required `CORPUSLAB_DATASET_ID` repository variable. `CORPUSLAB_CONFIG_LABEL` is optional and defaults to `default`.
8. Copy `docs/examples/github-actions-corpuslab-evals.yml` into `.github/workflows/corpuslab-evals.yml`.

The Actions runner must be able to reach the CorpusLab API. Set `CORPUSLAB_API_URL` to that reachable base URL. If CorpusLab is only available on a private local network, use a self-hosted runner and set the variable to the address reachable from that runner.

## Validate A Checked-In Golden Dataset

A workflow can validate and import a reviewed schema v1 file before running the existing CI gate. The import route uses the same `ci_eval_runs` key scope and target workspace as CI execution; it does not add a second CI architecture.

For an existing target dataset, validate first:

```sh
export GOLDEN_DATASET_FILE=evals/release-gate.json
export CORPUSLAB_DATASET_ID=018f7a2a-6e2e-7000-a000-000000000001

VALIDATION_JSON=$(curl --fail-with-body --silent --show-error \
  -H "Authorization: Bearer $CORPUSLAB_API_KEY" \
  -H "Content-Type: application/json" \
  --data-binary "@$GOLDEN_DATASET_FILE" \
  "$CORPUSLAB_API_URL/api/v1/eval-lab/ci/datasets/import?mode=merge_by_case_key&target_dataset_id=$CORPUSLAB_DATASET_ID&dry_run=true")

test "$(printf '%s' "$VALIDATION_JSON" | jq -r '.valid')" = true
VALIDATION_TOKEN=$(printf '%s' "$VALIDATION_JSON" | jq -er '.validation_token')
```

Review or record only the bounded counts and validation codes needed by the job; do not print the whole response because the imported file and validation details may identify private cases. Apply the exact validated file and target state:

```sh
curl --fail-with-body --silent --show-error \
  -H "Authorization: Bearer $CORPUSLAB_API_KEY" \
  -H "Content-Type: application/json" \
  --data-binary "@$GOLDEN_DATASET_FILE" \
  "$CORPUSLAB_API_URL/api/v1/eval-lab/ci/datasets/import?mode=merge_by_case_key&target_dataset_id=$CORPUSLAB_DATASET_ID&dry_run=false&validation_token=$VALIDATION_TOKEN"
```

Then submit the ordinary `/api/v1/eval-lab/ci/runs` request using `CORPUSLAB_DATASET_ID`. Use `create_new` without `target_dataset_id` when the workflow should create a dataset and read the new `dataset_id` from the apply response. `replace_dataset` additionally requires `confirm_replace=true`; `validate_only` cannot apply. A concurrent target change invalidates the token with `409`, requiring a fresh dry run. Unresolved or ambiguous checksum references, cross-workspace evidence, and full-local provenance fail validation instead of being guessed or downgraded.

## What The Example Evaluates

The example does not check out, execute, deploy, or otherwise evaluate the pull request's modified RAG application code. It requests an evaluation from the existing CorpusLab instance at `CORPUSLAB_API_URL`, using the configured Eval Lab dataset and that instance's current corpus, index, and retrieval configuration.

Branch, commit SHA, base/head refs, and configuration label are recorded as run metadata. The configuration label groups comparable runs; it does not apply a retrieval configuration by itself. If multiple pull requests target the same unchanged CorpusLab instance and dataset configuration, they may evaluate the same underlying system state.

Each CI-created experiment also captures a versioned immutable identity snapshot of the dataset revision, corpus/document checksums, chunking, chunks, embedding configuration/index, retrieval modes, `top_k`, scoring/filters, and runtime flags. CI and build values are informational and do not make otherwise identical retrieval inputs incompatible. Raw documents, queries, paths, chunk text, vectors, credentials, and response bodies are never copied into provenance.

To evaluate candidate-specific behavior, connect the workflow to a candidate-specific CorpusLab deployment, corpus/index, or retrieval configuration, then set `CORPUSLAB_API_URL`, `CORPUSLAB_DATASET_ID`, and `CORPUSLAB_CONFIG_LABEL` to identify that target.

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
- `400 Bad Request`: the dataset exceeds the shared 250-case experiment execution limit.
- `422 Unprocessable Entity`: run saved, but the gate failed and `fail_on_gate=true`.
- `401 Unauthorized`: missing, invalid, or revoked API key.
- `403 Forbidden`: API key does not include the `ci_eval_runs` scope.

The response includes:

- linked Eval Lab experiment
- gate status
- branch and commit metadata
- complete Eval Lab v2 regression and compatibility details, including machine-readable changed fields and newly failed, recovered, changed-top-evidence, and changed-failure-label cases, when a fully compatible baseline exists; otherwise a no-baseline comparison
- report JSON

The example writes only gate status, aggregate recall, failed-case count, opaque run/experiment IDs, and the configured label to logs and the Actions job summary. It deliberately never prints the response body because it may contain case queries and report content.

## Workbench Views

CI run history appears in:

- `/app/evals?view=ci-runs` under CI Runs.
- `/app/evals/ci-runs/:runId` as a focused detail view with revision metadata, thresholds, failed metrics, newly failing and recovered cases, failure labels, per-mode metrics, and report creation.
- Mission Control as latest gate status and recommended action.
- `/app/reports` as failed-gate report candidates with native CI audit-report creation.

Creating a report from a failed CI gate preserves the CI run source, branch, commit, configuration label, regression deltas, newly failed cases, and gate outcome. The action defaults to `metadata_only` and opens the generated report directly after creation.

CI baseline selection is strict: corpus, dataset, chunking, chunk-set, embedding index, ranking, scoring, filter, or runtime-flag changes exclude the run from automatic regression comparison. Use Experiment Detail's explicit baseline selector when a deliberate cross-configuration directional comparison is useful; CI does not silently promote that comparison to a normal regression.

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
