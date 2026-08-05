# ADR 0006: Bounded Autonomous Engineering

- Status: Accepted
- Date: 2026-08-05

## Context

CorpusLab needs sustained work across RAG correctness, frontend quality, performance, architecture, security, and testing. Unbounded agents with repository write credentials could interpret untrusted pull-request content, weaken their own checks, expose credentials, create noisy work, or merge changes without accountable review.

## Decision

Use independent planner and builder workflows with repository-owned prompts, policy, JSON schemas, sanitization, deterministic guardrails, and human-controlled approval labels. GPT-5.6 Sol runs the planner at `xhigh` reasoning and builder at `high` reasoning, one invocation per eligible run with no fallback or retry.

Codex generation is isolated from publication. It runs through the pinned official action with workspace-write and `drop-sudo`, receives no GitHub App token, and produces schema-constrained output plus a candidate artifact. A fresh credential-free job applies and tests the artifact with trusted code copied before candidate application. A separate publisher verifies an attestation and creates Git objects with a short-lived repository-only GitHub App token.

The planner can create proposed issues but cannot approve them. The builder can open one draft pull request but cannot mark it ready, approve, merge, deploy, release, repair, or close work. Protected paths are unavailable to autonomous changes, sensitive paths need an additional maintainer label, and size limits fail closed. The reviewed Issue #27 bootstrap carries a policy-owned, event-bound capability for a fixed list of exact CI-key, Settings, report, and privacy-document files. It cannot be created by issue or model content and does not weaken ordinary sensitive approval.

Candidate data is captured through bounded no-follow handles and immutable byte snapshots. Handle/path identity and version metadata is checked after each read, and the same bytes are scanned, hashed, copied, validated, and published. The publication client constructs only canonical HTTPS GitHub API destinations and refuses redirects.

## Consequences

The system can plan and implement bounded work while human review remains the authority. Failures are visible and require deliberate recovery. Full validation is slower and model use has cost, but concurrency, proposal caps, one-call runs, disabled schedules, and no retries bound that cost.

Repository source and sanitized issue requirements cross the OpenAI provider boundary. Runtime CorpusLab/customer data and publication credentials do not. The GitHub App adds a credential-rotation and installation-review obligation.

## Alternatives Considered

- A single workflow with one broad token was rejected because generated code could reach publication credentials.
- `GITHUB_TOKEN` write permissions were rejected because they would couple generation and publication and make least privilege harder to audit.
- Automatic approval or merge was rejected because CI success is not product, security, or architectural acceptance.
- A third-party orchestration service was rejected because repository-owned policy, schemas, tests, and audit history are easier to inspect and roll back.
- A lower-cost planner model was rejected for the initial baseline in favor of GPT-5.6 Sol `xhigh`; cost is bounded and must be reviewed after the one-time bootstrap.
