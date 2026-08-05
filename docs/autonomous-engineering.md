# Bounded Autonomous Engineering

CorpusLab uses a repository-owned planner and builder to prepare engineering work for human review. The automation is deliberately bounded: it may propose issues and open draft pull requests, but it cannot approve, mark ready, merge, deploy, release, repair CI or reviews, or close issues.

## Operating Model

The planner and builder are separate GitHub Actions workflows with separate concurrency groups and publication steps.

The planner runs weekly only when `AUTONOMY_PLANNER_SCHEDULE_ENABLED=true`. It reads trusted repository files plus an inventory containing issue numbers, titles, and labels. It returns at most three schema-validated proposals. A publisher labels them `agent/proposed`; a maintainer must deliberately add `agent/approved` before the builder can act.

The builder handles exactly one issue. An authorized approval-label event is the normal trigger. A six-hour reconciliation schedule, disabled unless `AUTONOMY_BUILDER_RECONCILE_ENABLED=true`, recovers eligible approvals that did not receive a run; it is not a retry mechanism. Priority is deterministic: P0 through P3, approval time, then issue number. `workflow_dispatch` performs diagnostics only and never calls Codex or publishes changes.

Exactly one open pull request labeled `agent/generated` is allowed. Generated branches use `agent/issue-<number>-<slug>`, and generated pull requests remain drafts with `Refs #<number>`. Issues remain open until a human accepts the work.

## Model Policy And Cost

Both roles use GPT-5.6 Sol in standard reasoning mode:

- Planner: `gpt-5.6-sol`, `xhigh` reasoning.
- Builder: `gpt-5.6-sol`, `high` reasoning.

There is one model invocation per eligible run, no fallback model, and no automatic retry. The planner creates no more than three proposals and stops when three proposed issues are already open. The builder processes one issue and stops after generation, validation, or publication failure. Configure a dedicated OpenAI project, service credential, budget, and usage alerts before enabling the workflows. Review usage after the Issue #27 bootstrap before enabling schedules.

## Trust Boundary

Trusted inputs are limited to policy, prompts, schemas, source, and engineering guidance already merged into `main`; the selected base commit; and the sanitized title/body of an authorized open issue. Planner issue titles are sanitized, bounded, untrusted duplicate-detection data, never instructions. Comments, review text, commit messages, hidden HTML, linked content, and external pages are not supplied as authority. Sanitization removes links, hidden markup, control characters, mentions, and secret-like values and caps builder issue input at 12,000 characters.

The official OpenAI Codex action receives repository source and sanitized engineering requirements. This is an external engineering-data boundary. It does not receive CorpusLab runtime documents, chunks, embeddings, retrieval queries, traces, reports, sessions, database contents, deployment credentials, or customer workspaces. Do not paste customer data or credentials into issues intended for autonomous processing.

Codex runs with the workspace-write sandbox and `drop-sudo`. Generation receives the OpenAI credential but no GitHub App token. Planner modifications are rejected. Builder changes are checked for secret-like content before they can enter a hashed, one-day artifact, then validated from a fresh checkout of the exact base SHA.

Validation receives no repository write credential. It rejects protected paths, unauthorized sensitive paths, unsafe paths, symlinks, generated artifacts, secret-like content, stale base commits, malformed structured output, missing tests, and changes above policy limits. It runs the complete Rust, web, Playwright, documentation, migration, Postgres contract, and Cargo Deny gates from a trusted script preserved before candidate application.

Only the publisher receives a short-lived repository GitHub App token. It never executes candidate code. It verifies the attestation and file hashes, creates Git objects through the canonical GitHub API, opens a draft pull request, and applies bounded labels. Candidate and artifact files are read through no-follow file handles into immutable byte snapshots, preventing symlink swaps or file changes between validation, hashing, and copying.

## GitHub App And Secrets

Create a GitHub App named for the CorpusLab autonomous engineer and install it only on `MuneebHoda/RAG-Debugger`. Disable webhooks and request only:

- Metadata: read.
- Contents: read/write.
- Issues: read/write.
- Pull requests: read/write.

Do not grant Actions, checks, deployments, environments, secrets, administration, members, or ruleset-bypass permissions.

Configure repository secrets:

- `OPENAI_API_KEY`: dedicated OpenAI project/service credential.
- `AUTONOMY_APP_ID`: GitHub App ID.
- `AUTONOMY_APP_PRIVATE_KEY`: current GitHub App private key.

Configure repository variables:

- `AUTONOMY_PAUSED=false`
- `AUTONOMY_APPROVERS=MuneebHoda`
- `AUTONOMY_PLANNER_SCHEDULE_ENABLED=false`
- `AUTONOMY_BUILDER_RECONCILE_ENABLED=false`

The pause variable fails closed: when `AUTONOMY_PAUSED` is missing, both workflows behave as though it were `true`. Deliberately set it to `false` only after the App, secrets, approver list, OpenAI budget, and alerts are ready.

The workflows create or reconcile the `agent/*` labels during the reviewed bootstrap. Repository label definitions remain versioned in `.github/labels.yml`.

## Approval And Change Policy

Only allowlisted collaborators with write, maintain, or admin permission may authorize work. Scheduled reconciliation verifies the latest approval-label event from the issue timeline. The same provenance rule applies to `agent/sensitive-approved` and `agent/large-approved`.

Protected paths always require manual development. They include GitHub configuration, autonomous policy and runtime, repository governance, agent/security guidance, the release guide, and quality entrypoints. Authentication, sessions, API keys, dependency manifests, migrations, storage ownership, environment configuration, report/export behavior, and privacy guidance or boundaries require `agent/sensitive-approved`. The versioned policy names these paths explicitly and fails closed on unclassified approval state.

More than 30 files or 2,000 meaningful non-generated lines requires written atomicity, testing, and rollback justification. More than 50 files or 4,000 lines requires `agent/large-approved`. The absolute limit is 100 files or 10,000 lines. Only deterministic paths listed in policy, currently the versioned handbook PDF, receive generated-file treatment.

## Bootstrap And Enablement

Complete App, secret, variable, and OpenAI budget setup before merging Issue #99. The builder recognizes only the `main` push that first introduces the reviewed authorization marker. If every preflight passes, that event claims Issue #27 once. Later pushes cannot recreate the bootstrap; claim/generated state also prevents duplication.

Preflight verifies that the OpenAI and GitHub App secrets are present before claiming or spending model tokens. If the initial bootstrap run stops at this setup check, configure the missing secret and manually rerun that original workflow event. Once Issue #27 is claimed, generation and validation failures remain non-retryable and require the documented label-reset recovery flow.

Review the Issue #27 draft, its diff, CodeRabbit feedback, required CI, privacy notes, and model cost. Leave both schedules disabled until this trial is accepted. Enable schedules by deliberately changing their repository variables; label-triggered builds remain available independently.

## Pause, Recovery, And Audit

Set `AUTONOMY_PAUSED=true` and cancel active planner/builder runs for an immediate stop. Selection, generation, deterministic validation, and publication each recheck the pause value in their own job; every GitHub write boundary checks it again. The App token is short-lived and revoked by the token action after each job.

A failed claimed attempt receives `agent/blocked` and does not retry. Review the workflow logs, remove the stale generated branch if publication stopped partway, correct the underlying issue or policy, remove `agent/blocked` and `agent/claimed`, then deliberately reapply `agent/approved`. Do not rerun a failed workflow as an implicit repair loop.

Audit evidence lives in Actions runs, issue-label timelines, claim/block comments, immutable candidate/attestation hashes during the one-day artifact window, generated commits, and draft pull requests. Logs contain identifiers and stage outcomes, not raw issue bodies, model prompts, credentials, or candidate bodies.

Rotate `OPENAI_API_KEY` in the OpenAI project and replace the GitHub repository secret. Rotate the GitHub App private key by adding the new key, replacing `AUTONOMY_APP_PRIVATE_KEY`, verifying a diagnostic run, and revoking the old key. App ID rotation requires updating the App installation and secret together.

## Rollback

Pause first. Cancel active runs, remove the GitHub App installation, delete the three secrets, and disable both schedule variables. Revert the autonomous-engineering commit through a reviewed pull request. Generated draft branches and pull requests can be closed or deleted manually. No CorpusLab application data, migration, API, branch protection, deployment, or release rollback is required.
