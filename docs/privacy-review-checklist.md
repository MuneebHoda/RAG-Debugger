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

## CI Eval Workflow Review Note

Issue #27 adds no external provider, telemetry, sharing destination, or new persisted data class. CI keys remain workspace-scoped, use the existing one-way hash storage, are shown once, and can be revoked only from their owning workspace. CI run metadata is bounded and rejects control characters. The GitHub Actions example keeps the key in an Actions secret and emits only gate status, aggregate counts/metrics, opaque IDs, and a config label; it never prints the CI response body, case queries, report bodies, documents, snippets, credentials, headers, or cookies. Failed-gate reports still default to `metadata_only`, apply existing redaction, and remain workspace-scoped.

## Immutable Experiment Provenance Review Note

Issue #60 adds a derived field to the existing workspace-scoped experiment JSON and no external provider, hosted path, telemetry, retention class, or migration. The snapshot contains opaque workspace/project/source/document/chunk identifiers, document and chunk checksums, canonical aggregate fingerprints, configuration, counts, and bounded build/CI metadata. It never copies raw queries, document paths, section titles, document/chunk text, embedding vectors, prompts, answers, credentials, headers, cookies, database URLs, or provider payloads; paths, chunk text, and section titles contribute SHA-256 fingerprints only. Manual and CI evaluation reuse one consistent workspace-scoped source/document/chunk/embedding snapshot acquired under one MemoryStore mutex or one Postgres read-only `REPEATABLE READ` transaction. Both stores validate provenance workspace/project ownership and reject duplicate experiment IDs.

Metadata-only reports include only aggregate fingerprints, counts, embedding configuration/index identity, compatibility, and fixed changed-field names. They exclude per-document checksums and all content fields. Provenance-build and bounded result-allocation failures emit only a generated correlation ID, opaque workspace/dataset IDs, a bounded requested case count, and the internal error; no raw corpus content, query, path, vector, request body, or credential reaches those events. Tests serialize provenance and reports with synthetic secret markers and assert that raw query/document content and document checksums do not enter provenance/report outputs respectively. Legacy experiments remain readable and are never silently classified as compatible. Rollback removes the additive producer/UI logic; stored JSON remains readable because provenance is optional.

## Golden Dataset Import/Export Review Note

Issue #61 adds an explicit authenticated local file boundary and no external provider, hosted sync, telemetry, sharing service, or new retention class. Full schema v1 exports may contain Eval queries, notes, evidence checksums, and provenance, so the workbench places the full download inside a clear warning disclosure and tells users to review files before sharing. Metadata-only export removes dataset description, queries, notes, and evidence references. Neither mode includes raw document/chunk text, snippets, paths, embeddings, credentials, headers, cookies, dataset/case/evidence UUIDs, or timestamps; optional provenance may identify its source trace. Datasets containing full-local case provenance cannot use full export.

Session export/import remains scoped to the authenticated workspace. CI import reuses the existing hashed `ci_eval_runs` API-key scope. Document and chunk checksums resolve only through workspace-owned project/source/document joins or equivalent MemoryStore ownership indexes, require one unambiguous match, and never fall back to a foreign ID, path, or text. Full-local provenance imports only when an authenticated same-workspace source trace, source/privacy classification, and exact retained query match; CI rejects it. Dry runs perform no write, apply tokens bind the exact canonical file and target revision, and MemoryStore/Postgres apply a valid plan atomically. Import/export handlers add no content logging; authenticated validation responses echo only the case keys and requested checksums necessary to repair the supplied file.

## Hard-Coded Credential Remediation Review Note

The API now requires a non-empty bootstrap password from the process environment and requires `DATABASE_URL` whenever Postgres is selected. Committed examples leave the bootstrap password blank; the documented fixed Postgres account is explicitly limited to the local Docker service. The login UI starts with empty fields and exposes no credentials. API and Playwright credentials are synthetic, named test fixtures, and no password, database URL, session value, or new diagnostic field is logged or returned through runtime configuration.

## CodeRabbit Review Provider Note

CodeRabbit is an external engineering review provider installed only on `MuneebHoda/RAG-Debugger`. It may process repository source, pull request diffs, review conversations, commit metadata, and CI status needed to produce advisory reviews. It is not integrated with CorpusLab runtime services and receives no documents, chunks, embeddings, retrieval queries, traces, reports, workspace records, database values, session values, API keys, headers, cookies, or deployment secrets from the product.

Raw customer data remains local by default, no product export or telemetry path changes, and no new data class is persisted by CorpusLab. Repository secrets remain prohibited in source, pull request content, logs, and test artifacts; GitHub secret scanning and push protection are enabled before CodeRabbit receives a pull request diff.

This public repository explicitly accepts CodeRabbit's default cache and knowledge-base retention. Code/dependency caches expire within seven days; review learnings and pull request context remain until an administrator deletes them or enables the immediate, irreversible `knowledge_base.opt_out`. The repository owner is responsible for reviewing CodeRabbit's maintained subprocessor register, currently including model providers such as OpenAI and Anthropic, and for verifying deletion in the CodeRabbit dashboard during rollback. After deletion is verified, removing repository access and reverting `.coderabbit.yaml` stops future processing without a CorpusLab data migration. ADR 0005 records this boundary and procedure.

## Local Trace Ingestion Review Note

- Data remains inside the configured CorpusLab API and database; no provider or external model is contacted.
- Workspace authority comes only from a valid session or hashed scoped API key; the project is verified inside that workspace.
- Native privacy is bounded by project policy. OTLP is forced to `metadata_only`, regardless of telemetry attributes.
- Filtering occurs before persistence, diagnosis, UI, Eval Lab, reports, or operational events. Metadata strips all content; snippets mode retains only explicit bounded evidence snippets. Tests assert secret markers are absent from stored restricted fields, errors, and captured operational logs.
- Operational events contain numeric counts, payload bytes, latency, mapping status, and stable reason codes only, using the existing local log destination and retention policy.
- Metadata/snippet Eval conversion remains denied because no query is retained. Full-local native conversion resolves the source inside the authenticated workspace, exact-matches the retained query, derives immutable provenance server-side, and requires authorization to access CorpusLab evidence. Provenance survives local experiment snapshots; CI and report creation reject affected datasets/experiments before persistence, so report Markdown, clipboard, download, and API export receive none of that imported content. No public sharing or trace clipboard integration was added.

## Private-Alpha Deployment Architecture Review Note

Issue #102 documents a future provider boundary; it does not provision or transmit customer data. The approved topology names GitHub/GHCR, Cloudflare Pages/Access/Tunnel, Render private services, and Render Postgres and explicitly records what each can technically observe. Raw content remains local by default; hosted upload is deliberate and disclosed, `full_local_only` transfer stays forbidden, and production data cannot become preview/staging fixtures. Environment secrets are distinct, provider-managed, unavailable to pull requests/build jobs, and prohibited from source/logs/artifacts.

Each environment's app and API hostnames share one default-deny multi-domain Access application with eager redirect cookies, avoiding a second API-host login without adding browser-visible service credentials. The Access cookie remains distinct from CorpusLab authentication. The only Access bypass is unauthenticated `OPTIONS`; API-host preflights still traverse the private tunnel, exact-origin API CORS remains authoritative, all non-`OPTIONS` requests remain Access-gated, and #107 must prove allowed/disallowed preflights, Access denial, provider-origin non-bypass, and continued CorpusLab authentication.

Hosted config validation fails before database connection or listener startup for insecure origins/cookies, local/default database settings, memory storage, external auth/embedding claims, verbose logs, mutable release identity, and oversized uploads. Errors name only the unsafe variable and requirement. No secret value, database URL, account content, Access identity, or provider payload is logged. Production remains inactive until #103–#108 implement the immutable artifacts, isolated infrastructure, qualification, redacted telemetry, retention, backup, restore, and incident evidence required by the contract.
