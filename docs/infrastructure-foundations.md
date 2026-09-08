# Private-Alpha Infrastructure Foundations

This is the deterministic desired-state contract for Issue #105. The machine-
checked source of truth is [`infra/private-alpha.json`](../infra/private-alpha.json).
It complements [ADR 0010](adr/0010-private-alpha-deployment.md). If this page and
the JSON disagree, validation enforces the machine-checkable identity and
isolation rules from the JSON; ADR 0010 remains authoritative for architecture.

No provider resource is created by this repository change. The operator domain,
one US Render region, provider IDs, human Access allowlist, production reviewer,
and all credential values are deliberately absent. They must be supplied through
the provider dashboards and the matching protected GitHub Environment. The JSON
status is `desired_state_only`; it never asserts external provisioning state.

## Required Operator Inputs

Before creating a resource, the maintainer records these non-secret decisions in
the private infrastructure record and matching GitHub Environment variables:

| Input                  | Allowed value                                        | Repository representation                                          | Provider location                                                 |
| ---------------------- | ---------------------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------- |
| Operator domain        | An operator-controlled registrable domain            | `{operator_domain}`; never invent a value                          | Cloudflare zone and `OPERATOR_DOMAIN` in both GitHub Environments |
| Render region          | Exactly one of `oregon`, `ohio`, or `virginia`       | `RENDER_REGION`; both environments use the same selected US region | Every Render API, connector, and Postgres resource                |
| Production reviewer    | A maintainer other than the deployment initiator     | `{production_reviewer}`                                            | GitHub `production` required reviewer                             |
| Human alpha allowlist  | Explicit identities or an IdP group                  | Provider-managed marker only                                       | Each Cloudflare Access application                                |
| Connector image digest | Reviewed Linux AMD64 `cloudflare/cloudflared` digest | `{approved_cloudflared_digest}`                                    | Both Render connector image references                            |

Cloudflare processes requests at its edge. Selecting a US Render region locates
the API and database but does not promise single-region ingress processing or a
contractual data-residency regime.

## Cloudflare Resource Model

| Resource               | Staging                         | Production                         | Required controls                                                                                                                  |
| ---------------------- | ------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Pages project          | `corpuslab-staging-web`         | `corpuslab-production-web`         | Direct Upload only; no Git/source build; no PR preview; redirect the default `pages.dev` hostname to the protected custom hostname |
| Web hostname           | `app.staging.{operator_domain}` | `app.{operator_domain}`            | Proxied DNS, managed edge certificate, HTTP-to-HTTPS redirect                                                                      |
| Access application     | `corpuslab-staging-access`      | `corpuslab-production-access`      | Exactly one self-hosted application per environment with exactly its concrete web and API hostname; no wildcard                    |
| Human Access policy    | Environment-specific allowlist  | Environment-specific allowlist     | Default deny; no public catch-all; provider administrators use MFA                                                                 |
| Access cookie behavior | Eager redirect cookie enabled   | Eager redirect cookie enabled      | Authentication at the web host prepares host-specific authorization cookies for both hosts                                         |
| Access bypass          | `OPTIONS` only                  | `OPTIONS` only                     | Every non-`OPTIONS` request remains Access-gated; Axum exact-origin CORS remains authoritative                                     |
| Tunnel                 | `corpuslab-staging-tunnel`      | `corpuslab-production-tunnel`      | One hostname ingress to the matching private API; terminal `http_status:404` rule                                                  |
| Connector identity     | `corpuslab-staging-cloudflared` | `corpuslab-production-cloudflared` | Unique tunnel token, image-backed Render worker, immutable cloudflared digest                                                      |
| API hostname           | `api.staging.{operator_domain}` | `api.{operator_domain}`            | Proxied tunnel route; no direct/provider API hostname                                                                              |

Cloudflare Access admits a private-alpha participant; it does not replace
CorpusLab login, session, workspace, role, or API-key authorization. Automation
uses an environment-specific Access service token and a separately scoped
CorpusLab API key. Browser JavaScript receives neither token.

Cloudflare documents [Direct Upload](https://developers.cloudflare.com/pages/get-started/direct-upload/),
[multi-domain eager cookies](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/#multi-domain-applications),
[the OPTIONS bypass](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/cors/#bypass-options-requests-to-origin),
and [redirecting `pages.dev`](https://developers.cloudflare.com/pages/how-to/redirect-to-custom-domain/).

## Render Resource Model

Create one Render project, `corpuslab-private-alpha`, with protected,
network-isolated `corpuslab-staging` and `corpuslab-production` environments.
Enable **Block cross-environment connections** on both. Keep preview
infrastructure disabled. Render documents that isolation prevents private
network traffic from crossing environment boundaries in its
[project environment controls](https://render.com/docs/projects#blocking-cross-environment-traffic).

| Resource                | Staging                                            | Production                                               | Ceiling and boundary                                                                                                                                                                              |
| ----------------------- | -------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| API private service     | `corpuslab-staging-api`                            | `corpuslab-production-api`                               | `pserv`, prebuilt `ghcr.io/muneebhoda/rag-debugger@sha256:<manifest digest>`, Linux AMD64, `0.5c-512mb`, exactly one instance, autoscaling off, no disk, no source build, no public Render origin |
| Tunnel worker           | `corpuslab-staging-cloudflared`                    | `corpuslab-production-cloudflared`                       | `worker`, prebuilt cloudflared digest, Linux AMD64, `0.5c-512mb`, exactly one instance, autoscaling off, no source build                                                                          |
| Postgres                | `corpuslab-staging-postgres` / `corpuslab_staging` | `corpuslab-production-postgres` / `corpuslab_production` | Postgres 17, 1 GiB, storage autoscaling off, empty public IP allowlist, same selected region as services, TLS required                                                                            |
| Database plan/recovery  | Free, synthetic-only, no backup claim              | `0.1c-256mb`, paid                                       | Production must show seven-day PITR on a Pro workspace before approved data; staging expires and is rebuildable                                                                                   |
| Runtime database role   | `corpuslab_staging_runtime`                        | `corpuslab_production_runtime`                           | Connect, schema usage, required DML/sequence use, read `_sqlx_migrations`; no schema creation/ownership                                                                                           |
| Migration database role | `corpuslab_staging_migration`                      | `corpuslab_production_migration`                         | Explicit `migrate` command only; schema DDL and SQLx migration-table ownership                                                                                                                    |
| Backup identity         | None on free synthetic staging                     | `render_managed_corpuslab_production_postgres`           | Provider control plane only; distinct from runtime and migration identities                                                                                                                       |

Render private services support TCP health checks rather than an HTTP health
path. Configure the provider check on port `8080`; deployment qualification must
call `https://api.../readyz` through Access/Tunnel and require a successful
database/migration check before traffic is accepted. Both long-running services
retain a 30-second graceful-shutdown window.

The API image is always selected from the trusted release manifest by digest.
Neither a branch, tag, `latest`, Render Git integration, nor a provider Docker
build is a deployment selector. Runtime config supplies the manifest source SHA
separately. The environment-specific web `runtime-config.js` remains outside the
immutable web application checksum.

### Database Role Bootstrap

Use the Render provider owner once, over TLS, to create unique random passwords
for the two named login roles. Store the runtime URL only in the matching Render
API secret environment and the migration URL only in the matching GitHub
Environment. Never put either URL in a Blueprint, shell history, issue, PR, log,
or support output. If no provider-private administrative path is available,
temporarily allow only the operator's exact IP for this bootstrap session,
confirm TLS, then restore the empty public IP allowlist immediately.

The provider owner grants the runtime role only database `CONNECT`, schema
`USAGE`, required table DML, sequence use, and `SELECT` on `_sqlx_migrations`.
The migration role receives database `CONNECT`, schema `USAGE, CREATE`, DDL
ownership, and SQLx migration-table access. Set migration-role default
privileges so future tables and sequences grant the runtime role only the
permissions the API needs. Verify the runtime role cannot create or alter a
table, then revoke routine provider-owner use. This preserves the packaged
fail-closed migration/startup contract without committing generated passwords.

## Runtime Variables And Secrets

The full runtime inventory and validation rules remain in
[deployment architecture](deployment-architecture.md#runtime-configuration-inventory).
The foundation fixes these hosted values in the matching Render API:
`RAG_DEBUGGER_ENV`, bind host/port, Postgres storage, exact HTTPS web/API
origins, hosted deployment mode, manifest source SHA, `info` logging, local
auth/embeddings, secure environment-specific `__Host-` cookie, a 168-hour
session ceiling, and the existing 10-file/20-MiB/50-MiB upload ceilings.

Private non-secret bootstrap identity fields (`RAG_DEBUGGER_BOOTSTRAP_EMAIL`,
`RAG_DEBUGGER_BOOTSTRAP_USER_NAME`, `RAG_DEBUGGER_BOOTSTRAP_ORGANIZATION`, and
`RAG_DEBUGGER_BOOTSTRAP_WORKSPACE`) belong only in the matching Render
environment. Their actual values are not repository defaults for hosted use.

### GitHub Environment Inventory

Create `staging` and `production` with the same non-secret variable names.
`OPERATOR_DOMAIN`, `RENDER_REGION`, the Cloudflare account/zone, and the Render
project are intentionally shared. Every environment, Pages, Access, tunnel,
service, connector, and database ID must differ:

- `OPERATOR_DOMAIN`, `RENDER_REGION`;
- `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_ZONE_ID`,
  `CLOUDFLARE_PAGES_PROJECT_ID`, `CLOUDFLARE_ACCESS_APPLICATION_ID`, and
  `CLOUDFLARE_TUNNEL_ID`;
- `RENDER_PROJECT_ID`, `RENDER_ENVIRONMENT_ID`, `RENDER_API_SERVICE_ID`,
  `RENDER_CONNECTOR_SERVICE_ID`, and `RENDER_DATABASE_ID`.

Store only these environment-specific secrets in GitHub:

| Staging secret                            | Production secret                            | Consumer and scope                                                                                      |
| ----------------------------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `CLOUDFLARE_STAGING_API_TOKEN`            | `CLOUDFLARE_PRODUCTION_API_TOKEN`            | Later environment deployment job; least-privilege Pages/DNS/Tunnel/Access mutation for that environment |
| `RENDER_STAGING_API_TOKEN`                | `RENDER_PRODUCTION_API_TOKEN`                | Later environment deployment/migration orchestration; never API runtime                                 |
| `DATABASE_STAGING_MIGRATION_URL`          | `DATABASE_PRODUCTION_MIGRATION_URL`          | Packaged `migrate` command only; distinct migration role; TLS required                                  |
| `CLOUDFLARE_STAGING_ACCESS_CLIENT_ID`     | `CLOUDFLARE_PRODUCTION_ACCESS_CLIENT_ID`     | Later qualified automation identity; sent only to Access                                                |
| `CLOUDFLARE_STAGING_ACCESS_CLIENT_SECRET` | `CLOUDFLARE_PRODUCTION_ACCESS_CLIENT_SECRET` | Same narrow Access client; never browser-visible                                                        |

The production Environment allows selected branch `main` only, requires an
independent maintainer approval, prevents self-review, and disallows
administrative bypass. Staging also selects only `main` but has no manual
approval. Environment secrets are unavailable until a job explicitly names its
matching Environment. The repository validator rejects production secret names
from every pull-request, build, or staging job and rejects any pull-request job
that targets a production resource or obtains deployment-write permission.

### Render-Managed Secret Inventory

| Secret name                       | Store/consumer                               | Isolation rule                                                                                             |
| --------------------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `DATABASE_URL`                    | Matching Render API secret environment       | Runtime-role URL only; distinct host/database/user/password per environment; `sslmode=require` or stronger |
| `RAG_DEBUGGER_BOOTSTRAP_PASSWORD` | Matching Render API secret environment       | At least 16 characters and unique per environment; rotate after bootstrap or suspected exposure            |
| `TUNNEL_TOKEN`                    | Matching Render connector secret environment | Issued for exactly one tunnel; never shared across environments                                            |
| `GHCR_PULL_TOKEN`                 | Matching Render registry credential store    | Separate read-packages-only identity per environment; never available to application code                  |

The provider-created database owner and production PITR identity stay in Render.
No service receives provider-owner database access or broad provider admin
credentials.

## Data, Logs, Backup, And Cost Boundaries

- Staging contains synthetic data, or sanitized data with an explicit owner and
  one-way review. Application rows are removed within 30 days. Production is
  never cloned or automatically restored into staging.
- Production contains only approved private-alpha data. Before any such data,
  the maintainer verifies paid PITR and #108 must prove an isolated restore.
- Hosted logs contain only fields allowed by the logging-redaction contract.
  Render Pro retains logs for 14 days. That meets production's target but cannot
  implement staging's shorter seven-day target per environment; staging remains
  synthetic-only and #108 owns the retention/export evidence before activation.
- Pages projects use the Cloudflare Free ceilings of 20,000 files per project,
  25 MiB per file, and 500 builds per month. The two projects consume two of the
  documented 100-project account limit. Access Free is $0 for teams under 50
  users; crossing that bound requires cost review.
- The current Render estimate is **$59/month before bandwidth, storage, taxes,
  or price changes**: Pro workspace $25, four `0.5c-512mb` services at $7 each,
  production `0.1c-256mb` Postgres $6, and free staging Postgres $0. The free
  staging database is limited to 1 GiB, expires after 30 days, and has no backup.
- The alpha budget stays at $100/month. Review actual provider usage at $75 and
  before any plan, storage, instance, Access-seat, or bandwidth increase. The
  providers do not expose one aggregate hard cap for this stack, so the review
  is an operator control, not a claimed automatic budget lock.

Current facts come from the official [Render pricing](https://render.com/pricing),
[Render free database limits](https://render.com/docs/free#free-postgres),
[Render backups](https://render.com/docs/postgresql-backups),
[Render log retention](https://render.com/docs/logging#retention),
[Cloudflare Pages limits](https://developers.cloudflare.com/pages/platform/limits/),
and [Cloudflare Access pricing](https://www.cloudflare.com/sase/products/access/).
Recheck those pages at provisioning time.

## Maintainer Provisioning Checklist

Check off an item only after saving its non-secret output in the matching GitHub
Environment or private infrastructure record. Never paste a secret into an
issue, PR, repository file, command output, or this checklist.

| Action required                                  | Provider            | Exact setting/resource to create                                                                                                                                                                                       | Expected non-secret identifier/output                                                                         | Secret name, if any                                                                           | Secret location                                                                    | Verification                                                                                                                                   | Environment |
| ------------------------------------------------ | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| Select inputs                                    | Operator / GitHub   | Controlled domain, one allowed US `RENDER_REGION`, independent production reviewer, two human allowlists                                                                                                               | Domain, region, reviewer, allowlist owner recorded privately; `OPERATOR_DOMAIN` and `RENDER_REGION` variables | None                                                                                          | GitHub Environment variables / private record                                      | Values satisfy the Required Operator Inputs table; region is identical for each service/database                                               | Both        |
| Create isolated Render containers                | Render              | Pro workspace project `corpuslab-private-alpha`; protected `corpuslab-staging` and `corpuslab-production`; Block cross-environment connections on both                                                                 | Project ID and two distinct environment IDs                                                                   | None                                                                                          | Matching GitHub Environment variables                                              | Provider UI shows protection and isolation enabled; cross-environment private connection test fails                                            | Both        |
| Create databases                                 | Render              | Named Postgres 17 resources, selected region, stated plan, 1 GiB, storage autoscaling off, empty public IP allowlist                                                                                                   | Two database IDs and distinct internal endpoints                                                              | Provider owner generated by Render                                                            | Render only                                                                        | IDs/endpoints differ; external connection is denied; TLS connection in same environment succeeds                                               | Both        |
| Bootstrap database roles                         | Render Postgres     | Distinct runtime and migration roles/grants; production provider-managed PITR identity                                                                                                                                 | Four distinct role names plus production recovery page/window                                                 | `DATABASE_STAGING_MIGRATION_URL`, `DATABASE_PRODUCTION_MIGRATION_URL`; runtime `DATABASE_URL` | Migration URLs in matching GitHub Environment; runtime URLs in matching Render API | Runtime role performs DML and reads `_sqlx_migrations` but cannot DDL; migration role runs packaged migration; URLs require TLS                | Both        |
| Add GHCR readers and API private services        | Render / GHCR       | Distinct read-packages registry credentials; two image-backed `pserv` services using the release-manifest digest, one instance, no public origin/source build                                                          | Two API service IDs and digest shown by Render                                                                | `GHCR_PULL_TOKEN`, `DATABASE_URL`, `RAG_DEBUGGER_BOOTSTRAP_PASSWORD`                          | Matching Render registry/API secret stores                                         | Image digest equals manifest; TCP 8080 healthy; no `onrender.com`/public API route; unsafe runtime config fails                                | Both        |
| Create tunnels and connectors                    | Cloudflare / Render | Two named remotely managed tunnels; two image-backed Render workers using approved cloudflared digest and matching private API ingress; terminal 404                                                                   | Two tunnel IDs and two connector service IDs                                                                  | `TUNNEL_TOKEN`                                                                                | Matching Render connector secret store                                             | Each tunnel has its own connected connector; unmatched ingress is 404; staging connector cannot reach production private API and vice versa    | Both        |
| Create Pages sites                               | Cloudflare          | Two named Direct Upload projects, one custom web hostname each, previews unused, `pages.dev` redirected to custom hostname                                                                                             | Two Pages project IDs and two custom-hostname statuses                                                        | None at runtime                                                                               | IDs in matching GitHub Environment                                                 | Project has no Git integration; HTTPS custom hostname works; `pages.dev` cannot serve content around Access                                    | Both        |
| Configure DNS/TLS/Access                         | Cloudflare          | Proxied DNS for four hosts; managed certificates/HTTP redirect; one self-hosted two-domain Access app per environment; eager cookie on; default-deny human allowlist; `OPTIONS` bypass only; environment service token | Four active DNS records/certificates; two Access application IDs; two service-token client IDs                | Environment Cloudflare API token and Access client secret                                     | Matching GitHub Environment                                                        | Signed-out non-`OPTIONS` denied; exact two hosts/no wildcard; eager sequence completes; `OPTIONS` reaches Axum; CorpusLab login still required | Both        |
| Create protected deployment environments         | GitHub              | `staging` and `production`; selected branch `main`; production required reviewer, prevent self-review, no admin bypass; populate only matching variables/secrets                                                       | Two Environment URLs and protection-rule JSON                                                                 | All five environment-specific GitHub secrets listed above                                     | Matching GitHub Environment only                                                   | GitHub API shows rules; a PR/build/staging job cannot read production secrets; production job waits before secrets are released                | Both        |
| Configure recovery, retention, and cost controls | Render / Cloudflare | Verify production seven-day PITR; provider notifications; monthly usage review at $75; named billing/recovery owner; staging 30-day data deletion schedule                                                             | Recovery window, notification recipients, owner, and dated budget record                                      | None                                                                                          | Provider/private operating record                                                  | Restore control exists only for production; staging shows no backup claim; calculated base stays under $100; free limits recorded              | Both        |
| Record provisioning evidence                     | Operator            | Sanitized IDs/statuses and verification results on Issue #105; no secrets or customer data                                                                                                                             | Completed checklist with provider timestamps and failure-free validation                                      | None                                                                                          | Issue/private record as appropriate                                                | Another maintainer can match every resource to `infra/private-alpha.json`; #105 remains open until this evidence exists                        | Both        |

## Repository Validation

Run:

```sh
cd apps/web
npm run governance:test
node scripts/validate-infrastructure.mjs
```

The check validates both environments, every current workflow, exact host and
Access shape, isolation identities, immutable image selection, TLS, private API
ingress, database roles/endpoints, resource ceilings, backup claims, retention,
cost bounds, secret-name separation, credential-value absence, and the
pull-request/production mutation boundary. It uses the already installed
Prettier YAML parser; no infrastructure framework or new dependency is added.

Provider API validation cannot run before the operator supplies accounts,
domain, region, IDs, and credentials outside the repository. After provisioning,
rerun this repository check and complete the provider-side verification column.
Issue #106 may then consume the IDs and secrets through protected Environments;
this issue does not add a deployment workflow or deploy an artifact.
