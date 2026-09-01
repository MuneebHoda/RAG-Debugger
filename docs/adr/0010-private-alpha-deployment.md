# ADR 0010: Private-Alpha Deployment Architecture

- Status: Accepted
- Date: 2026-09-01
- Issue: [#102](https://github.com/MuneebHoda/RAG-Debugger/issues/102)

## Context

CorpusLab runs locally as a Vite application, an Axum API, and Postgres. The API currently owns local startup migrations, uses one exact credentialed CORS origin, issues host-only `SameSite=Lax` session cookies, and stores corpus-derived data in Postgres. The repository has source CI but no production image, artifact publication, hosted infrastructure, deployment workflow, or operational control plane.

A small private alpha needs a reviewable path to hosted operation without turning the local-first product into a public SaaS, exposing production data to pull requests, rebuilding different artifacts per environment, or adding Kubernetes and other speculative infrastructure.

## Decision

The reference hosted-alpha stack is:

- GitHub Actions for trusted quality gates, artifact publication, staging deployment, and maintainer-approved production promotion;
- GitHub Container Registry (GHCR) for the API image, selected only by immutable digest;
- Cloudflare Pages Direct Upload for the prebuilt Vite artifact;
- Cloudflare DNS, managed TLS, Access, and Tunnel as the private-alpha ingress boundary;
- a `cloudflared` connector and image-backed Render private service for the Axum API; and
- Render Postgres in the same selected region as the API, with separate staging and production instances.

Cloudflare [accepts direct uploads of prebuilt Pages assets](https://developers.cloudflare.com/pages/get-started/direct-upload/). Render [accepts prebuilt private GHCR images and digest selectors](https://render.com/docs/deploying-an-image). Those capabilities let GitHub build once and promote verified artifacts instead of asking hosting providers to rebuild source.

The logical topology is:

```text
browser
  ├─ HTTPS → app.<environment>.<operator-domain>
  │             Cloudflare Access + Pages
  └─ HTTPS → api.<environment>.<operator-domain>
                Cloudflare Access + Tunnel
                  → cloudflared connector
                    → private Axum API service
                      → TLS → private Render Postgres
```

The web and API use sibling HTTPS origins under the same operator-owned registrable domain. This preserves the implemented credentialed cross-origin model and avoids adding a path-routing proxy. The API permits exactly the corresponding web origin; wildcard origins are forbidden. The API session cookie remains host-only, `HttpOnly`, `SameSite=Lax`, `Secure`, and uses a distinct `__Host-` name per environment.

The immutable web bundle loads a small public runtime configuration before application startup. It contains only API origin, environment label, and release SHA. Issue #103 will implement that loader. `VITE_API_BASE_URL` remains for local/test builds and must not encode staging or production into the promoted bundle.

Cloudflare terminates browser TLS. The Cloudflare Tunnel is the only production route to the private API service; the API has no public Render hostname. The application does not trust or make authorization decisions from `Forwarded` or `X-Forwarded-*` headers. Cloudflare owns client-IP and original-scheme interpretation, must overwrite forwarded headers, and must not forward them into application logs. Any future application use of forwarded headers requires a separate trusted-proxy implementation and tests.

Cloudflare Access is a default-deny alpha admission layer, not CorpusLab authorization. CorpusLab local authentication, workspace sessions, roles, and scoped API keys remain authoritative inside the application. Browser users pass both layers. Automation uses a separately scoped Access service token plus the existing CorpusLab API key. The connector and API are private so bypassing Access through a provider default hostname is not possible.

## Ownership

| Concern | Owner |
| --- | --- |
| Source, review, CI gates, protected environments | GitHub |
| API image, SBOM, provenance, release manifest | GitHub Actions and GHCR |
| Web artifact upload, DNS, ingress TLS, alpha admission | Cloudflare |
| API scheduling and private connector runtime | Render |
| Database encryption, availability, snapshots/PITR capability | Render |
| Migration approval, releases, rollback, incidents, retention policy | Named CorpusLab maintainer |

GitHub build jobs receive no staging or production runtime secrets. GitHub Environment secrets become available only in the matching deployment job; production requires maintainer approval. Cloudflare and Render service credentials are distinct between staging and production.

## Environments And Data

Local, pull-request/test, staging, and production are separate. Pull requests run only ephemeral local services with synthetic fixtures. Staging uses synthetic or explicitly sanitized fixtures. Production data is never cloned, restored, or exported into a lower environment by default. Staging and production have distinct Pages projects, Access applications, tunnels/connectors, Render environments, API identities, databases, credentials, domains, cookie names, logs, backups, and release records.

The chosen US Render region is recorded during #105 and shared by each environment's API and database. Cloudflare processes ingress at its edge, so the reference architecture does not promise single-region request processing or formal data residency. A partner requiring contractual residency, a different jurisdiction, or customer-managed keys cannot join this alpha until a reviewed provider/region change satisfies that requirement.

## Build, Migration, And Rollback

One trusted commit produces one API digest and one static web artifact checksum. Staging qualifies them; production receives those same identities. Branch names and `latest` are not deployment selectors. Environment runtime configuration is separate from the immutable artifact and its safe checksum is recorded with the deployment.

Hosted migrations are explicit pre-deploy work performed by a dedicated migration identity. The runtime API identity has data access but no schema ownership. Startup-coupled migrations remain acceptable only for local development until #103 adds the packaged migration command and hosted startup switch. #106 will serialize deployments, create or verify a recovery point, run forward-only migrations, require readiness and canaries, and stop promotion on failure.

Application rollback redeploys a retained last-known-good image digest and matching web checksum. Applied migrations are never automatically reversed. A migration that is not compatible with both the new and last-known-good application requires an expand/migrate/contract plan before production approval.

## Availability And Cost Profiles

The low-cost evaluation profile may use sleeping/free compute and an expiring free database only with synthetic data. It has no SLA, no managed database backup, cold starts, provider retention limits, and a monthly budget ceiling of USD 25. It is not production and cannot receive approved alpha corpus data. Render documents that free Postgres expires and has no managed backups in its [service](https://render.com/docs/service-types) and [free-tier](https://render.com/docs/free) guidance.

The private-alpha production profile requires always-on paid API/connector capacity and paid Postgres with verified recovery capability. Its initial operator budget ceiling is USD 100 per month, with an alert at 75 percent. Paid Render Postgres provides plan-dependent PITR rather than making CorpusLab itself highly available; see [Render recovery behavior](https://render.com/docs/postgresql-backups). The alpha still makes no contractual SLA.

Before promising stronger availability, #105/#108 must select multi-instance connector/API capacity, HA database capability, support/alert delivery, and demonstrated restore behavior. Upgrade review is mandatory when any of these occurs: more than five active design partners, a required availability target above 99.5 percent, RPO below 24 hours, RTO below four hours, repeated cold-start/capacity failures, sustained 70 percent resource or connection use, exhausted storage/retention limits, or a forecast above the USD 100 ceiling.

## Alternatives Considered

- **One VM with Docker Compose:** fewer vendors, but makes patching, TLS, backups, isolation, and recovery operator-owned. Rejected for the hosted alpha; retained for local development.
- **Git-connected provider builds:** convenient, but rebuild source separately and weaken artifact provenance. Rejected.
- **Same-origin edge routing:** simplifies CORS and runtime API discovery, but adds a Worker/reverse-proxy deployment and another content-processing runtime. Deferred unless cross-origin qualification proves unreliable.
- **Public API service behind application auth only:** cheaper, but leaves provider-origin bypass and public signup exposure. Allowed only for synthetic no-SLA evaluation, never production data.
- **Serverless API functions:** poorly match the Axum process, bounded uploads, SQLx pools, and deliberate migrations. Rejected.
- **Kubernetes, service mesh, queue, Redis, multi-region, and GPU workers:** unnecessary for the private-alpha load and operational capacity. Deferred.
- **Provider-neutral specification only:** would leave #105 unable to provision a deterministic stack. The provider choice is explicit, while replacements remain possible through a superseding ADR if they meet the same contract.

## Consequences And Deferred Work

Cloudflare can observe request content after TLS termination; Render can observe API process data and persisted database content. This is disclosed in the deployment specification. Raw content remains local unless a user deliberately uses the hosted deployment with approved data. No automatic local-to-hosted sync is introduced.

This ADR does not create images, publish artifacts, provision resources, deploy an environment, enable production, add observability, or implement backup workflows. Those changes remain assigned to #103–#108 as mapped in [the deployment architecture](../deployment-architecture.md#follow-up-issue-map).
