# ADR 0006: Bounded Autonomous Engineering

- Status: Accepted
- Date: 2026-08-05

## Context

CorpusLab needs sustained work across RAG correctness, frontend quality, performance, architecture, security, and testing. Unbounded agents with repository write credentials could interpret untrusted pull-request content, weaken their own checks, expose credentials, create noisy work, or merge changes without accountable review.

## Decision

Use independent planner and builder workflows with repository-owned prompts, policy, JSON schemas, sanitization, deterministic guardrails, and human-controlled approval labels. GPT-5.6 Sol runs the planner at `xhigh` reasoning and builder at `high` reasoning, one invocation per eligible run with no fallback or retry.

Codex generation is isolated from publication. It runs through the pinned official action with workspace-write and `drop-sudo`, receives no GitHub App token, and produces schema-constrained output plus an untrusted candidate artifact. A fresh trusted job validates canonical context and every candidate byte before executing candidate code, writes a complete attestation, and directly uploads one immutable sealed file. GitHub's artifact ID and digest are propagated as workflow outputs, and overwrite is forbidden.

A disposable quality job downloads that exact artifact by ID, verifies the digest, reapplies it to the exact base, and runs the complete gate with no write credential, App token, OIDC permission, publication secret, or artifact promotion. A separate fresh publisher downloads the original artifact, repeats schema, authorization, policy, hash, size, secret, path, and exact-base validation, and executes no candidate code. Only after revalidation does it mint a short-lived token restricted to `MuneebHoda/RAG-Debugger` and run trusted publication code.

The planner can create proposed issues but cannot approve them. The builder can open one draft pull request but cannot mark it ready, approve, merge, deploy, release, repair, or close work. Protected paths are unavailable to autonomous changes, sensitive paths need an additional maintainer label, and size limits fail closed. The reviewed Issue #27 bootstrap carries a policy-owned, event-bound capability for a fixed list of exact CI-key, Settings, report, and privacy-document files. It cannot be created by issue or model content and does not weaken ordinary sensitive approval.

Candidate data is captured through bounded no-follow handles and immutable byte snapshots. Handle/path identity and version metadata is checked after each read, and the same bytes are scanned, hashed, sealed, quality-tested from a disposable copy, revalidated, and published. Canonical `context.json` and publication metadata are schema-bound to trusted preflight state. The supported JSON Schema subset is explicit and fails closed on unsupported keywords. The publication client constructs only canonical HTTPS GitHub API destinations and refuses redirects.

## Consequences

The system can plan and implement bounded work while human review remains the authority. Failures are visible and require deliberate recovery. Full validation is slower and model use has cost, but concurrency, proposal caps, one-call runs, disabled schedules, and no retries bound that cost.

Repository source and sanitized issue requirements cross the OpenAI provider boundary. Runtime CorpusLab/customer data and publication credentials do not. The GitHub App adds a credential-rotation and installation-review obligation.

## Alternatives Considered

- A single workflow with one broad token was rejected because generated code could reach publication credentials.
- `GITHUB_TOKEN` write permissions were rejected because they would couple generation and publication and make least privilege harder to audit.
- Automatic approval or merge was rejected because CI success is not product, security, or architectural acceptance.
- A third-party orchestration service was rejected because repository-owned policy, schemas, tests, and audit history are easier to inspect and roll back.
- A lower-cost planner model was rejected for the initial baseline in favor of GPT-5.6 Sol `xhigh`; cost is bounded and must be reviewed after the one-time bootstrap.
