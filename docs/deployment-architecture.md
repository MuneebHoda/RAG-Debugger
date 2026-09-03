# Private-Alpha Deployment Architecture

This document is the environment and deployment contract for CorpusLab's small hosted private alpha. [ADR 0010](adr/0010-private-alpha-deployment.md) records the provider and topology decision. This contract does not deploy anything: issues #103–#108 implement it.

## Current State And Approved Target

Today the API runs from Cargo, the web app runs from Vite, Docker Compose starts only local Postgres, migrations run inside API startup, and `VITE_API_BASE_URL` is compiled into the web bundle. `/healthz` reports process liveness; `/readyz` pings storage and deliberately reports unavailable in the `test` environment. CORS permits one configured credentialed web origin. The API ignores forwarded headers and emits privacy-safe startup and trace-ingestion events, but does not yet provide hosted JSON request telemetry, packaged artifacts, deployment automation, backups, or provider resources.

Issue #102 adds a first-class `staging` runtime value, release-SHA labeling, and fail-closed staging/production validation. It does not add infrastructure. The approved target is the following bounded stack:

- GitHub Actions and protected GitHub Environments;
- GHCR API images selected by digest;
- Cloudflare Pages Direct Upload for immutable static web assets;
- Cloudflare DNS, TLS, Access, and Tunnel;
- an image-backed Render `cloudflared` connector and private Axum API service; and
- isolated Render Postgres instances.

## Approved Topology

```text
                                  GitHub Actions
                             build, attest, promote
                                      │
                         ┌────────────┴────────────┐
                         │                         │
                    GHCR digest              web artifact
                         │                         │
                         ▼                         ▼
browser ── HTTPS ── Cloudflare Access/TLS/DNS ── Pages
   │                     │
   └── credentialed fetch to api.<environment>.<operator-domain>
                         │
                         ▼
                 Cloudflare Tunnel
                         │
                         ▼
              Render cloudflared connector
                         │ private network
                         ▼
                   Axum API image
                         │ PostgreSQL TLS
                         ▼
              environment-owned Postgres
```

Web and API are sibling origins under one operator-owned registrable domain:

```text
staging:    https://app.staging.<operator-domain>
            https://api.staging.<operator-domain>
production: https://app.<operator-domain>
            https://api.<operator-domain>
```

Names above are templates, not committed provider IDs. #105 records the actual domain and region in protected environment configuration.

Each environment has exactly one Cloudflare Access self-hosted multi-domain application with two concrete domains and the same default-deny human policy:

```text
staging Access application:
  app.staging.<operator-domain>
  api.staging.<operator-domain>

production Access application:
  app.<operator-domain>
  api.<operator-domain>
```

The applications must not use wildcard hostnames. Cloudflare's [Eager redirect cookie](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/#multi-domain-applications) setting is enabled so authentication at the app hostname preemptively issues a host-specific `CF_Authorization` cookie for both concrete hostnames. The browser can then call the API with `credentials: include` without first visiting the API hostname, encountering a second login, or adding a Worker/service credential to browser traffic.

### Request And Trust Flow

1. Cloudflare Access denies users not on the private-alpha allowlist before Pages or the API tunnel is reached.
2. A successful login at the app hostname completes the multi-domain application's eager redirect sequence and sets `CF_Authorization` for both concrete hosts.
3. Pages returns the immutable Vite bundle. A public runtime configuration supplies only API origin, environment, and release SHA.
4. The browser calls the sibling API origin with `credentials: include`; Access accepts the already-issued API-host cookie without a second login.
5. The API CORS policy names one exact HTTPS web origin, permits credentials, and never uses `*`. CorpusLab login then returns its separate host-only `__Host-` session cookie with `HttpOnly`, `Secure`, `SameSite=Lax`, and `Path=/`.
6. Cloudflare carries API traffic through an outbound tunnel connector. The Render API is private and has no provider-default public origin.
7. The API connects to the same-environment Postgres instance using required TLS and the runtime database role.

Cloudflare Access admits an alpha participant; it does not replace CorpusLab authentication or workspace authorization. Its cookie is not accepted as a CorpusLab session. CorpusLab sessions, roles, repository ownership checks, and API-key scopes remain authoritative. Browser users authenticate at both boundaries. CI and collectors use an Access service token plus a separately scoped CorpusLab API key.

Browsers do not send cookies on preflight requests. The Access application therefore enables Cloudflare's [bypass OPTIONS requests to origin](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/cors/#bypass-options-requests-to-origin) setting. This bypass is limited to `OPTIONS`; API-host preflights still reach Axum only through the private tunnel. Axum's existing CORS layer remains the single policy source: it returns credentialed preflight headers only for the configured web origin, and disallowed origins receive no approval. Access still gates every non-`OPTIONS` API request, and CORS is never `*`.

### TLS And Trusted Proxies

Cloudflare terminates browser TLS. Tunnel transport protects Cloudflare-to-connector traffic. Connector-to-API traffic remains on the Render private network; API-to-Postgres uses PostgreSQL TLS even on that network. HTTP is redirected to HTTPS at the edge.

The application currently does not consume `Forwarded`, `X-Forwarded-For`, or `X-Forwarded-Proto`, and hosted authorization must not depend on them. Cloudflare owns the client address and original scheme, strips or overwrites inbound forwarded headers, and keeps them out of application logs. Adding application-level client-IP or proxy trust requires an allowlisted-hop implementation and qualification tests in a later reviewed change.

## Environment Contract

| Environment | Purpose and permitted data | Trigger and approval | Origins | Isolation and identity | Retention |
| --- | --- | --- | --- | --- | --- |
| Local | Development. Developer-owned local or synthetic data; private content stays on that machine. | Developer command; no approval. | Web `http://127.0.0.1:5173`; API `http://127.0.0.1:8080`. | Ignored `.env`; local Docker Postgres or MemoryStore; cookie `corpuslab_session`; exact localhost CORS; `environment=local`, release `development`. | Developer controlled. No backup is implied. |
| Test/PR/preview | Repository qualification with checked-in synthetic fixtures only. There is no remotely deployed PR preview by default. | Pull request; CI gates only; never deployable. | Playwright web `http://127.0.0.1:15173`; API `http://127.0.0.1:18080`; other jobs use ephemeral ports. | Synthetic credentials; MemoryStore or ephemeral CI Postgres; cookie `corpuslab_test_session`; exact test CORS; `environment=test` and untrusted commit identity; no staging/production secrets, database, tunnel, or Access identity. | Job lifetime. Sanitized CI logs/artifacts follow the repository's bounded GitHub retention setting. |
| Staging | Release rehearsal and synthetic canaries. Sanitized data requires an explicit owner and one-way export review; production clones are forbidden. | Trusted `main` artifact after all gates; automatic staging deployment; no production approval. | `app.staging.<operator-domain>` and `api.staging.<operator-domain>`. | Separate Cloudflare Pages project, one two-host multi-domain Access app with eager cookies, tunnel, connector, Render environment/API, database, runtime/migration roles, GitHub Environment, secrets, and `__Host-corpuslab_staging_session`. Exact staging CORS; `environment=staging`. | Rebuildable. Application data at most 30 days; privacy-safe logs at most 7 days unless an incident hold is approved. No dependency on production backups. |
| Production/private alpha | Approved alpha data only. Hosted raw uploads are an explicit partner choice; automatic local sync and `full_local_only` transfer are forbidden. | Promotion of the staging-qualified immutable artifacts; protected `production` Environment and maintainer approval. | `app.<operator-domain>` and `api.<operator-domain>`. | Separate production Pages project, one two-host multi-domain Access app with eager cookies, tunnel, connector, Render environment/API, paid database, runtime/migration roles, secrets, and `__Host-corpuslab_alpha_session`. Exact production CORS; `environment=production`. | Rows remain while the alpha workspace is active; removal and backup-expiry requests are operator-owned until #108 adds the runbook. Privacy-safe logs target 14 days. Paid PITR/recovery retention is recorded from the selected provider plan. |

Production data does not flow to test, previews, or staging by default. Restore drills target a new isolated non-production database and use synthetic data or a specifically approved, access-restricted production recovery copy; they never overwrite another environment. A production-derived recovery copy is not general staging data and is deleted after the drill.

### Environment Approval And Access Rules

- Pull-request code can read no hosted environment secret and can mutate no hosted environment.
- The staging deploy identity can update staging services only.
- The production deploy identity is released only after GitHub Environment approval and can update production services only.
- Production approval identifies the release owner, rollback owner, API digest, web checksum, runtime-config checksum, migration set, recovery point, and qualification run.
- Each environment's two concrete hostnames share one default-deny multi-domain Cloudflare Access application with eager redirect cookies enabled. Human policies list approved alpha identities; automation policies accept separately rotated service tokens.
- Provider dashboards are maintainer-only with MFA. Runtime services receive no broad provider administration credential.

## Runtime Configuration Inventory

Classifications:

- **Public build-time:** visible in static JavaScript; never secret.
- **Non-secret runtime:** provider/API setting safe to disclose, although customer-chosen labels may still be private metadata.
- **Secret runtime:** secret manager value; never in source, images, web assets, logs, or support output.
- **Local-only:** useful for developer/test commands and forbidden as hosted artifact configuration.

Hosted values are set in the matching Render service/environment or supplied to a protected deployment/migration job. Local values live in ignored `.env`; test values live in the individual CI job. `GET /api/v1/config` exposes only the typed product/UI subset, never server secrets.

### Process, Network, Storage, And Auth

| Variable | Class and consumer | Local/default | Staging/production contract | May log value? |
| --- | --- | --- | --- | --- |
| `RAG_DEBUGGER_ENV` | Non-secret API runtime | `local`; accepts `dev`, `development`, `test` | Required `staging` or `production`; legacy `prod` remains accepted; unknown values fail | Yes |
| `RAG_DEBUGGER_RELEASE_SHA` | Non-secret API runtime | `development` | Required full 40-character lowercase commit SHA | Yes |
| `RAG_DEBUGGER_LOG` | Non-secret API runtime | `info` | `info`, `warn`, or `error` directives only; `debug`/`trace` fail | Yes |
| `RAG_DEBUGGER_API_HOST` | Non-secret API runtime | `127.0.0.1` | Required non-loopback bind, normally `0.0.0.0` | Yes |
| `RAG_DEBUGGER_API_PORT` | Non-secret API runtime | `8080` | Required provider-assigned non-zero port | Yes |
| `RAG_DEBUGGER_WEB_ORIGIN` | Non-secret API runtime/CORS | `http://127.0.0.1:5173` | Required exact non-local HTTPS web origin with no path/query/fragment | Yes |
| `RAG_DEBUGGER_PUBLIC_API_BASE_URL` | Public API runtime config | `http://127.0.0.1:8080` | Required exact non-local HTTPS API origin with no path/query/fragment | Yes |
| `RAG_DEBUGGER_STORAGE_BACKEND` | Non-secret API runtime | `postgres`; `memory` permitted | Required `postgres`; MemoryStore fails | Yes |
| `DATABASE_URL` | Secret API runtime | Required for Postgres; Docker-only example credential | Required credentialed non-local PostgreSQL URL with `sslmode=require`, `verify-ca`, or `verify-full`; local default fails | Never |
| `RAG_DEBUGGER_DEPLOYMENT_MODE` | Non-secret API/product config | `hybrid` | Required `hosted` | Yes |
| `RAG_DEBUGGER_AUTH_PROVIDER` | Non-secret API runtime | `local` | Required `local`; `external` is only a boundary marker today and fails hosted validation | Yes |
| `RAG_DEBUGGER_SESSION_COOKIE_NAME` | Non-secret API runtime | `corpuslab_session` | Required distinct non-empty `__Host-` name | Yes |
| `RAG_DEBUGGER_SESSION_TTL_HOURS` | Non-secret API runtime | `168` | Required 1–168 | Yes |
| `RAG_DEBUGGER_SESSION_COOKIE_SECURE` | Non-secret API runtime | `false` | Required `true` | Yes |
| `RAG_DEBUGGER_BOOTSTRAP_EMAIL` | Private runtime identity metadata | local demo email | Environment-owned initial operator identity | No |
| `RAG_DEBUGGER_BOOTSTRAP_PASSWORD` | Secret API runtime | Required non-empty ignored value | Required secret with at least 16 characters, unique per environment | Never |
| `RAG_DEBUGGER_BOOTSTRAP_USER_NAME` | Private runtime identity metadata | `Demo User` | Environment-owned operator label | No |
| `RAG_DEBUGGER_BOOTSTRAP_ORGANIZATION` | Private runtime identity metadata | demo organization | Environment-owned organization label | No |
| `RAG_DEBUGGER_BOOTSTRAP_WORKSPACE` | Private runtime identity metadata | demo workspace | Environment-owned workspace label | No |

CorpusLab sessions and API-key secrets are generated randomly. Only SHA-256 lookup hashes are stored; hashes are also prohibited from logs. There is no session-signing secret because sessions are opaque server-side records.

### Product, Ingestion, Retrieval, And Embeddings

These values are non-secret API runtime configuration. Defaults are safe locally; hosted overrides are environment-specific configuration recorded with the release. Log the variable name, not its value, unless the table explicitly says the value is safe to log. Only the fixed product name and embedding provider/model/dimension are value-loggable here; customer workspace labels, tuning values, limits, and extension lists are not.

| Variable | Default | Hosted requirement |
| --- | --- | --- |
| `RAG_DEBUGGER_PRODUCT_NAME` | `CorpusLab` | Optional; fixed public product label is safe to log |
| `RAG_DEBUGGER_WORKSPACE_NAME` | `Corpus Workspace` | Optional; do not log a customer-chosen value |
| `RAG_DEBUGGER_MAX_FILES_PER_REQUEST` | `10` | Required range 1–10 |
| `RAG_DEBUGGER_MAX_FILE_BYTES` | `20971520` | Required range 1 byte–20 MiB |
| `RAG_DEBUGGER_MAX_REQUEST_BYTES` | `52428800` | Must cover one file and not exceed 50 MiB |
| `RAG_DEBUGGER_PREVIEW_CHUNK_LIMIT` | `8` | Optional bounded UI projection |
| `RAG_DEBUGGER_SUPPORTED_EXTENSIONS` | `txt,md,markdown,html,htm,pdf` | Optional allowlist; never add executable formats implicitly |
| `RAG_DEBUGGER_DEFAULT_TARGET_TOKENS` | `512` | Optional release configuration |
| `RAG_DEBUGGER_DEFAULT_OVERLAP_TOKENS` | `64` | Optional release configuration |
| `RAG_DEBUGGER_DEFAULT_CHUNKING_STRATEGY` | `structured` | Optional; `structured` or `whitespace` behavior |
| `RAG_DEBUGGER_DEFAULT_TOP_K` | `5` | Optional release configuration |
| `RAG_DEBUGGER_MAX_TOP_K` | `25` | Optional resource boundary |
| `RAG_DEBUGGER_DEFAULT_RETRIEVAL_MODE` | `hybrid` | Optional; `hybrid`, `vector`, or `lexical` |
| `RAG_DEBUGGER_MIN_EVIDENCE_SCORE` | `0.35` | Optional release configuration |
| `RAG_DEBUGGER_MIN_SEMANTIC_SIMILARITY` | `0.25` | Optional release configuration |
| `RAG_DEBUGGER_ANSWER_CITATION_LIMIT` | `3` | Optional resource boundary |
| `RAG_DEBUGGER_MIN_ANSWER_BODY_TERM_COVERAGE` | `0.50` | Optional validated ratio |
| `RAG_DEBUGGER_MIN_ANSWER_BODY_TERM_MATCHES` | `2` | Optional positive count |
| `RAG_DEBUGGER_LOW_SCORE_MARGIN_RATIO` | `0.10` | Optional validated ratio |
| `RAG_DEBUGGER_WEIGHT_SEMANTIC_HYBRID` | `2.0` | Optional release configuration |
| `RAG_DEBUGGER_WEIGHT_SEMANTIC_VECTOR` | `3.0` | Optional release configuration |
| `RAG_DEBUGGER_WEIGHT_LEXICAL` | `2.4` | Optional release configuration |
| `RAG_DEBUGGER_WEIGHT_FREQUENCY` | `0.6` | Optional release configuration |
| `RAG_DEBUGGER_WEIGHT_PHRASE` | `1.2` | Optional release configuration |
| `RAG_DEBUGGER_WEIGHT_SECTION` | `0.75` | Optional release configuration |
| `RAG_DEBUGGER_WEIGHT_PATH` | `0.5` | Optional release configuration |
| `RAG_DEBUGGER_WEIGHT_METADATA` | `1.0` | Optional release configuration |
| `RAG_DEBUGGER_EMBEDDING_PROVIDER` | `local` | Required `local` until an external-provider ADR exists; safe to log |
| `RAG_DEBUGGER_EMBEDDING_MODEL` | `local-hash-v1` | Optional model identity; safe to log |
| `RAG_DEBUGGER_EMBEDDING_DIMENSION` | `384` | Optional model identity; safe to log |
| `RAG_DEBUGGER_SHOW_LOCAL_BADGES` | `true` | Optional public UI flag |

### Web And Automation Values

| Value | Class, location, and rule |
| --- | --- |
| `VITE_API_BASE_URL` | Public build-time; local/test only. It currently selects the API origin in `apps/web/src/lib/api/client.ts`. Hosted artifacts must not bake an environment URL; #103 replaces hosted use with public runtime config. |
| Runtime `apiBaseUrl` | Public runtime, stored beside the deployed web artifact; exact HTTPS API origin. |
| Runtime `environment` | Public runtime; `staging` or `production`. |
| Runtime `releaseSha` | Public runtime; must match the promoted release manifest. |
| `CORPUSLAB_API_URL` | Non-secret CI/collector variable; target API origin. Safe to log. |
| `CORPUSLAB_API_KEY` | Secret CI/collector value; workspace-scoped CorpusLab bearer token. Never log. |
| `CORPUSLAB_PROJECT_ID` | Private non-secret collector metadata; opaque project ID. Log only when needed for safe correlation. |
| `CORPUSLAB_DATASET_ID` | Private non-secret CI metadata; opaque dataset ID. Log only when needed for safe correlation. |
| `CORPUSLAB_CONFIG_LABEL` | Private non-secret CI metadata; bounded label. Safe only under the existing redaction policy. |
| Cloudflare Access client ID/secret | Secret automation values in the matching GitHub Environment; sent only to Access, never to CorpusLab logs. |

`CORPUSLAB_CAPTURE_WORKBENCH` and `UPDATE_REPORT_FIXTURES` are local/test-only developer switches and are forbidden in hosted runtime configuration.

`PrivacyMode` is a per-project/trace/report domain policy, not an environment variable. Hosted configuration cannot weaken it globally: `metadata_only` remains the default transfer/report boundary, `snippets_allowed` remains explicit and bounded, and `full_local_only` remains ineligible for hosted transfer.

### Hosted Startup Validation

For `staging` and `production`, the API exits before database connection or listener startup unless all of these hold:

- API host/port, web origin, API origin, and release SHA are explicitly present;
- origins are non-local absolute HTTPS origins with no path, query, fragment, or user information;
- the bind address is non-loopback and the port is non-zero;
- storage is Postgres and `DATABASE_URL` is credentialed, non-local, named, and requires TLS;
- deployment mode is `hosted`, the implemented auth provider remains `local`, and embeddings remain local;
- cookies are `Secure`, use a non-empty `__Host-` name, and live 1–168 hours;
- the bootstrap password is at least 16 characters;
- logging does not enable `debug` or `trace`;
- the release identity is a full lowercase commit SHA; and
- upload count, file size, and request size stay inside the documented 10-file/20-MiB/50-MiB ceilings.

The validation never returns or logs a database URL, password, token, cookie, or rejected secret value.

## Provider And Data Processing Inventory

Provider access describes technical visibility, not permission to inspect customer data.

| System/provider | Can potentially observe | Must not receive or retain |
| --- | --- | --- |
| User browser/device | Anything the authenticated user uploads or views, plus session and Access cookies | Another workspace's data; secrets in persistent browser logs/storage |
| GitHub repository/Actions | Source, reviews, synthetic fixtures, commit/PR/CI metadata, build logs, protected deploy secrets in approved jobs, CI API responses in runner memory | Customer corpus, production database, raw hosted traces/reports, printed secrets, production secrets in PR/build jobs |
| GHCR/attestation store | API filesystem, image metadata, SBOM, provenance, digest | Runtime secrets, database values, customer data, environment-specific config |
| Cloudflare Pages | Static web files, public runtime config, deployment metadata | API secrets or corpus content in the static artifact |
| Cloudflare DNS/Access/Tunnel | DNS names, client/network metadata, Access identity, headers/cookies and HTTP request/response content after TLS termination, including uploads and queries that use hosted API | Provider analytics containing bodies, authorization, cookies, filenames, paths, queries, snippets, answers, or trace payloads |
| Render connector/API | Image identity, process/runtime metadata, all API request content in memory, generated responses, privacy-safe application logs | Secrets or corpus content in logs; public ingress that bypasses the tunnel |
| Render Postgres/backups | All persisted rows and derived content, encrypted storage, connection metadata | Original upload binaries, plaintext session/API-key secrets, data from another environment |
| Health/error monitor selected in #108 | Probe URL, status, latency, environment, release, sanitized error class | Authenticated routes, bodies, queries, corpus/eval/report content, credentials |

### CorpusLab Data Classes

- **Raw documents:** local by default. A browser upload to hosted alpha crosses Cloudflare and Render API only after explicit partner approval; original bytes are processed in memory and are not persisted. Extracted chunks are persisted.
- **Paths/names, chunks/snippets, embeddings, queries, answers, traces/spans, evidence, Eval datasets, reports:** sensitive. Hosted use makes them visible to the API provider and, when persisted, the database provider. In-transit API content is also technically visible at the Cloudflare boundary.
- **Experiment provenance and CI metadata:** derived workspace metadata. Provenance omits raw queries/text/paths/vectors, but IDs, checksums, fingerprints, branches, and commits remain private operational data.
- **Users/accounts:** emails, display names, organization/workspace labels, password hashes, sessions, roles, and API-key records persist in Postgres. Passwords are Argon2 hashes; session and API-key lookup values are SHA-256 hashes; full tokens are not persisted.
- **Credentials:** browser cookies and authorization headers cross Cloudflare and reach API memory. Deployment/database/provider secrets remain only in their secret managers and approved job memory.

`full_local_only` data is not eligible for hosted transfer. Metadata-only or explicitly snippets-allowed trace/report flows remain the preferred hosted boundary. There is no automatic corpus, trace, or report sync in this architecture.

## Secret Ownership

| Secret | Storage and consumer | Scope/rotation owner |
| --- | --- | --- |
| `DATABASE_URL` | Render secret environment; API runtime | One environment/runtime role; maintainer rotates |
| Migration database URL/credential | Protected GitHub Environment or one-shot Render job; migration command only | One environment/migration role; release owner rotates |
| Bootstrap password | Render secret environment; API bootstrap only | Unique per environment; alpha owner rotates after suspected exposure |
| Cloudflare API token | Protected deployment Environment; deployment job | Pages/DNS/Tunnel minimum permissions; infrastructure owner |
| Tunnel token/credential | Render connector secret | One environment/tunnel; infrastructure owner |
| Cloudflare Access service token | Protected automation environment/approved client | One automation purpose and environment; integration owner |
| Render API token | Protected deployment Environment | Environment/service mutation only where provider supports it; release owner |
| GHCR pull credential | Render registry credential store | Read packages only; release owner |
| CorpusLab API key | Calling repository/collector secret store | One workspace and scope; workspace owner |

Provider account IDs, service IDs, domain names, and release SHAs are not secrets, but remain environment-specific configuration and must not be confused with credentials.

## Build And Release Contract

```text
source commit
    ↓
quality/security gates
    ↓
single immutable build
    ↓
attested API image + static web artifact
    ↓
staging deployment
    ↓
qualification/canary
    ↓
maintainer-approved promotion
    ↓
same artifact in production
```

Required invariants:

1. Only a trusted `main` commit or reviewed version tag can publish. Pull requests build/test but never publish or deploy.
2. The API image is identified by GHCR digest. The web artifact has a cryptographic checksum. `latest`, branch names, and mutable provider builds are forbidden selectors.
3. One build produces staging and production inputs. Production never recompiles source or regenerates application bundles.
4. The release manifest binds commit SHA, API digest, web checksum, migration-set checksum, schema/application version, SBOM/provenance references, and workflow URL.
5. Public runtime web configuration is environment-specific, generated from protected configuration rather than source edits, and recorded by checksum beside the deployment.
6. Build jobs have no runtime/provider production secret. Deploy credentials appear only in the selected GitHub Environment.
7. Staging deployments cancel superseded runs. Production deployments serialize without canceling an in-progress promotion.
8. Readiness and synthetic canaries gate promotion. A bounded Codex builder never approves, merges, publishes, or deploys production.
9. GHCR retains every currently deployed and last-known-good digest. Artifact cleanup cannot remove rollback inputs.

### Release And Rollback Decisions

| Failure point | Required action | Production impact |
| --- | --- | --- |
| Source/security gate fails | Do not build/publish | None |
| Artifact verification fails | Quarantine manifest; do not deploy | None |
| Staging migration fails | Stop, preserve sanitized diagnostics, restore/fix staging deliberately | None |
| Staging readiness/canary fails | Keep last-known-good staging artifact; block production | None |
| Production migration fails before compatible completion | Stop deployment; release owner follows migration runbook/PITR decision | Existing app remains where schema permits |
| Production readiness/canary fails | Stop traffic switch or redeploy last-known-good compatible artifacts | Possible bounded outage; no automatic DB rollback |
| Privacy/auth/workspace isolation incident | Pause ingress/ingestion, revoke affected credentials, preserve sanitized evidence, use private security process | Alpha paused until owner clears restart |

## Database And Migrations Contract

- Staging and production use separate Postgres instances, credentials, network rules, backups, and migration histories. Database branches/schemas on one shared credential are insufficient isolation.
- The API runtime role receives only required DML/schema usage. A distinct migration role owns DDL and SQLx migration-table updates. Backup/restore uses a third provider/operator identity where supported.
- Current API startup runs SQLx migrations. That remains local-development behavior only. #103 must provide an explicit packaged migration command and disable startup-coupled hosted migration before production activation.
- #106 invokes migrations once, under deployment concurrency, before the new API becomes ready. Runtime replicas never race to migrate.
- Migrations are append-only and forward-only. Applied files are never edited, deleted, or reordered.
- Every schema change must be compatible with the currently running and last-known-good application, or declare an expand/migrate/contract sequence that prevents application rollback until safe.
- A failed migration blocks readiness/promotion and preserves the previous application. There is no unbounded retry and no automatic down migration.
- Staging rehearses empty-database and supported-prior-version migration. Production verifies a recent recoverable point before DDL.
- Private-alpha production requires provider PITR/backups and a verified isolated restore before approved data arrives. Initial targets for #108 are RPO no worse than 24 hours and RTO no worse than four hours; no SLA is claimed until drills prove them.
- Irreversible data loss or destructive migrations require a separate ADR and explicit maintainer approval. Application rollback after such a change is not assumed possible.

## Operational Baseline

These are activation requirements; #107/#108 implement and verify them.

- `/healthz` proves only that the process can respond. It must not query dependencies or expose version/secrets.
- `/readyz` must remain unavailable until Postgres is reachable and migrations are compatible. Render/Cloudflare routing and post-deploy qualification use it before user traffic.
- Logs are structured JSON in hosted environments and follow [the redaction policy](logging-redaction.md). Required fields are environment, release SHA/digest, route template, method, status class, duration, bounded byte/count fields, and approved opaque correlation IDs. Current text output is not the final hosted format.
- Monitoring probes only public health/readiness and synthetic canary routes. Alerts contain environment, release, status, duration, counts, and runbook link—never content.
- Minimum alerts cover API/readiness failure, elevated 5xx rate, database connection failure, failed migration/deploy/canary, stale backup, and failed restore verification.
- Paid production Postgres must expose backup/PITR capability. #108 records the actual window, validates a restore into isolation, and names the recovery owner.
- Provider log retention targets are 7 days staging and 14 days production. Application data and backup retention follow the environment table and verified provider plan; legal/partner requirements can only shorten or deliberately supersede them.
- Cost alerts fire at 75 percent of the environment budget. Maximum instances, database storage, egress, build minutes, Pages uploads, and log volume are capped where providers support it.
- One maintainer is deployment, rollback, privacy-incident, and provider-escalation owner for each release. The alpha pauses when no owner is available.

### #107 Access And CORS Qualification

#107 must run the following flow in a fresh standard browser session against the concrete staging hostnames:

```text
visit app hostname
    ↓
authenticate once through the environment's multi-domain Access application
    ↓
complete the eager redirect sequence and load the SPA
    ↓
SPA sends a credentialed request to the API hostname
    ↓
Access accepts the API-host cookie without another login or manual API visit
```

The first request targets the public CorpusLab configuration endpoint so success proves the Access/CORS path. The same qualification then verifies that a protected API route still returns the CorpusLab authentication response until the user logs in to CorpusLab. It must also prove:

- an allowed-origin credentialed preflight succeeds through the `OPTIONS`-only Access bypass with the exact staging web origin;
- a preflight from any other origin receives no CORS approval;
- a user outside the Access allowlist cannot reach any non-`OPTIONS` API route;
- no Render/provider-default hostname or direct connector address can reach the API around Access/Tunnel; and
- after Access admission, CorpusLab session and workspace authorization remain unchanged and required.

## Cost And Growth Boundary

The no-SLA evaluation profile is synthetic-only, may sleep or expire, has no managed backup guarantee, and is capped at USD 25/month. It is useful for proving #103–#107, not for production data.

The production private-alpha profile uses paid always-on connector/API resources and paid Postgres recovery capability. The initial total ceiling is USD 100/month with an alert at USD 75. Provider price changes are checked during #105 and recorded in the protected environment/release issue; this document does not promise current plan prices.

Move to stronger paid availability only after a measured trigger: more than five active partners, required uptime above 99.5 percent, RPO below 24 hours, RTO below four hours, persistent cold-start/capacity failures, sustained 70 percent resource/connection use, a provider storage/retention ceiling, or cost forecast above USD 100. That review may add multiple API/connector instances and HA Postgres, but not Kubernetes by default.

## Assumptions And Explicit Deferrals

- The private alpha is small, invite/Access-gated, single-region, and has no contractual SLA.
- Local hashing remains the only embedding provider. No external model or analytics provider is added.
- Hosted raw content is opt-in approved data, not an automatic extension of local storage.
- Cloudflare/Render replacement, residency guarantees, customer-managed keys, multi-region, public signup, billing, enterprise SSO, SCIM, complex RBAC, queues, and GPU workers require later decisions.
- Provider resource IDs, secrets, production values, Dockerfiles, infrastructure definitions, publishing/deployment workflows, monitors, and runbooks do not belong in #102.

## Follow-Up Issue Map

- **#103:** production API image, immutable static web artifact, runtime web config, explicit hosted migration command, graceful shutdown, and production-parity local stack.
- **#104:** trusted immutable GHCR/web publication, SBOM, provenance, scanning, attestations, and release manifest.
- **#105:** separate Cloudflare/Render staging and production resources, regions, domains, one concrete two-host multi-domain Access application per environment with eager cookies and `OPTIONS`-only bypass, tunnels, databases, roles, secrets, quotas, and provider configuration.
- **#106:** automatic staging deployment and maintainer-approved promotion of the same artifacts, with concurrency, migration, readiness, and rollback records.
- **#107:** packaged/staging qualification and bounded synthetic production canaries across TLS, the fresh-browser single-Access-login sibling-origin flow, exact preflight CORS, Access denial/bypass resistance, CorpusLab re-authentication, cookies, migrations, restart, and privacy behavior.
- **#108:** hosted JSON telemetry, alerts/SLOs, backup/PITR configuration, isolated restore evidence, retention/deletion procedures, incident/runbook ownership, and recovery drills.

Production promotion stays disabled until #107 succeeds against staging. Approved design-partner data stays out until #108 proves alert delivery, backup freshness, isolated restore, last-known-good artifact availability, and rollback ownership.
