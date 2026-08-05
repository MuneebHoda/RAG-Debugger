# Privacy Review Checklist

Privacy review is required whenever a change can alter where customer data moves, who can access it, how long it remains available, or what appears in diagnostics. The author completes this checklist before review; a reviewer other than the author confirms material privacy-boundary changes.

## Review Triggers

A pull request needs privacy review if it:

- uploads, syncs, exports, or shares raw document text;
- sends chunks, embeddings, queries, prompts, answers, citations, or traces to an external service;
- changes embedding, reranking, generation, OCR, analytics, or telemetry provider behavior;
- changes trace, report, eval, or corpus sync/export behavior;
- adds hosted, organization, workspace, collaboration, or report-sharing functionality;
- changes authentication, sessions, API keys, roles, retention, deletion, or access control;
- logs raw queries, chunks, prompts, answers, headers, cookies, credentials, or provider payloads; or
- changes the meaning of `PrivacyMode` or the local-first default.

## Data Movement

- [ ] Raw documents remain local by default.
- [ ] Uploaded binaries are discarded after extraction unless explicit retention is documented and approved.
- [ ] Raw chunks, vectors, queries, prompts, answers, and citations are not newly sent externally, or each destination and purpose is documented.
- [ ] External processing is explicit, scoped to the active workspace/project, and visible to the user.
- [ ] Exports and shares require an explicit user or CI action.
- [ ] Trace and report exports apply the documented redaction policy.
- [ ] Retention, deletion, and backup behavior is defined for every newly persisted data class.

## Secrets And Access

- [ ] Passwords, session tokens, cookies, API keys, authorization headers, provider secrets, and database URLs are never logged.
- [ ] Runtime bootstrap passwords and non-local database credentials come from deployment-managed environment values; committed examples contain no password except credentials explicitly scoped to local Docker services.
- [ ] API key and session secrets are stored only as one-way hashes; full API key secrets are shown once.
- [ ] New endpoints enforce the intended session, API-key scope, workspace, and role boundary.
- [ ] Error responses do not expose storage errors, SQL, credentials, internal paths, or provider payloads.
- [ ] Test fixtures and screenshots contain synthetic data only.

## Logs And Diagnostics

- [ ] Logs use opaque IDs, counts, durations, statuses, and approved short checksum prefixes.
- [ ] Query text is treated as sensitive unless a deliberate, documented opt-in permits capture.
- [ ] Raw document/chunk text, embedding vectors, prompt bodies, generated answers, and citation snippets are absent from logs.
- [ ] Debug logging cannot reveal secrets when enabled in production.
- [ ] New telemetry fields have a documented owner, purpose, retention period, and cardinality bound.

## Documentation And Decision Record

- [ ] `docs/privacy-security.md` remains accurate.
- [ ] `docs/logging-redaction.md` remains accurate.
- [ ] The relevant feature guide and technical handbook describe new data movement.
- [ ] An ADR records any hosted sync, external provider, retention, auth-provider, or privacy-boundary decision.
- [ ] `CHANGELOG.md` records user-visible privacy or security behavior.

Use `N/A` with a short reason in the pull request for checks that do not apply. A checked box means the author verified the behavior; it does not mean the feature has no privacy impact.

## Guided Demo Review Note

The guided demo uses only checked-in synthetic Markdown fixtures and local processing. Loading is an explicit authenticated action scoped to the active workspace. It persists ordinary extracted chunks and embeddings, never separate fixture binaries, and makes no external calls. Suggested query text is returned only to the authenticated client and is not placed in URLs. Audit reports default to `metadata_only`; `full_local_only` export remains blocked. Loading does not delete or reset customer data.

## Debugger Intelligence v2 Review Note

Structured diagnosis is derived locally from already persisted retrieval metadata. The diagnosis snapshot contains opaque IDs, ranks, scores, failure codes, counts, and deterministic remediation text; it does not copy raw queries, paths, section titles, snippets, chunk text, credentials, headers, cookies, or report bodies. No external call, new telemetry, sharing path, retention class, or authorization boundary is introduced. Metadata-only report tests verify that private source content cannot enter the diagnosis export, and full-local-only export remains blocked.

## Expected-Evidence Lookup Review Note

Evidence lookup remains an authenticated local workbench operation and introduces no external provider, telemetry, export, or retention class. Requested IDs resolve through workspace-scoped storage authorization and are capped before repository access. Candidate search returns only bounded metadata and 280-character chunk previews; unrestricted chunk bodies are not copied into evidence lookup responses. MemoryStore browse uses derived ordering keys without duplicating raw corpus text, while its local-development substring search reads canonical text under the existing process-local lock. Short non-ID searches are rejected, selected IDs remain removable when metadata resolution fails, and queries, previews, credentials, headers, cookies, and database values are not logged.

Issue #68 privacy review: protected middleware validates real sessions in tests and production; corpus evidence, embedding status/writes, retrieval candidates/runs, traces/reruns, Eval datasets/cases/experiments/runs, CI reads, demo progress, Overview metrics, and report inputs carry the authenticated workspace into repository calls. MemoryStore and Postgres enforce ownership before returning data or mutating evidence, embeddings, runs, or traces. Cross-workspace and nonexistent identifiers receive equivalent unresolved, unavailable, or resource-specific not-found results. Trace writes verify project and source-run ownership, while embedding ownership is derived from chunk ancestry. The migrations assign only uniquely attributable or single-workspace legacy records; ambiguous multi-workspace records remain quarantined. No raw corpus content, inaccessible IDs, membership details, cookies, or secrets are added to logs or errors.

## Hard-Coded Credential Remediation Review Note

The API now requires a non-empty bootstrap password from the process environment and requires `DATABASE_URL` whenever Postgres is selected. Committed examples leave the bootstrap password blank; the documented fixed Postgres account is explicitly limited to the local Docker service. The login UI starts with empty fields and exposes no credentials. API and Playwright credentials are synthetic, named test fixtures, and no password, database URL, session value, or new diagnostic field is logged or returned through runtime configuration.

## CodeRabbit Review Provider Note

CodeRabbit is an external engineering review provider installed only on `MuneebHoda/RAG-Debugger`. It may process repository source, pull request diffs, review conversations, commit metadata, and CI status needed to produce advisory reviews. It is not integrated with CorpusLab runtime services and receives no documents, chunks, embeddings, retrieval queries, traces, reports, workspace records, database values, session values, API keys, headers, cookies, or deployment secrets from the product.

Raw customer data remains local by default, no product export or telemetry path changes, and no new data class is persisted by CorpusLab. Repository secrets remain prohibited in source, pull request content, logs, and test artifacts; GitHub secret scanning and push protection are enabled before CodeRabbit receives a pull request diff.

This public repository explicitly accepts CodeRabbit's default cache and knowledge-base retention. Code/dependency caches expire within seven days; review learnings and pull request context remain until an administrator deletes them or enables the immediate, irreversible `knowledge_base.opt_out`. The repository owner is responsible for reviewing CodeRabbit's maintained subprocessor register, currently including model providers such as OpenAI and Anthropic, and for verifying deletion in the CodeRabbit dashboard during rollback. After deletion is verified, removing repository access and reverting `.coderabbit.yaml` stops future processing without a CorpusLab data migration. ADR 0005 records this boundary and procedure.

## Bounded Autonomous Engineering Review Note

The autonomous planner and builder send public repository source plus sanitized, maintainer-approved engineering issue text to OpenAI through the pinned Codex GitHub Action. Comments, reviews, commit messages, hidden HTML, external-link contents, secret-like values, and CorpusLab runtime/customer data are outside the trusted input. The workflows are not connected to product databases, workspaces, documents, chunks, embeddings, retrieval queries, traces, reports, sessions, API keys, headers, cookies, or deployment secrets.

Codex receives the dedicated OpenAI project credential through the action's protected proxy and `drop-sudo` path but never receives the GitHub App publication token. Validation runs without publication credentials. Only trusted publisher code receives a short-lived repository-only App token after deterministic tests and artifact attestation succeed. The App has contents, issues, and pull-request permissions only; it has no Actions, secrets, deployments, administration, or ruleset-bypass access.

Issue text is capped, sanitized, and never written to logs by repository scripts. Candidate artifacts contain repository changes and structured summaries, expire after one day, and are never product exports. Pause, credential removal, App uninstall, artifact expiry, and a reviewed commit revert stop processing without a CorpusLab migration. ADR 0006 and `docs/autonomous-engineering.md` record the complete boundary.
