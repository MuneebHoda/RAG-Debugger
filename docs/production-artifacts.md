# Production Artifacts

CorpusLab packages one non-root API image and one immutable Vite application
artifact. These artifacts implement the boundaries approved in
[ADR 0010](adr/0010-private-alpha-deployment.md); they do not publish or deploy
anything.

## API Image

Build the Linux AMD64 image from the locked workspace and identify it with the
source commit and release version:

```sh
docker buildx build --platform linux/amd64 --load \
  --build-arg OCI_REVISION="$(git rev-parse HEAD)" \
  --build-arg OCI_VERSION=0.1.0 \
  --tag corpuslab-api:local .
```

The final `scratch` stage contains only the statically linked API binary and CA
certificate bundle. It runs as numeric UID/GID `65532:65532`, exposes port
`8080`, uses the binary as its exec-form entrypoint, and includes OCI source,
revision, version, and MIT license labels. The image needs no writable root
filesystem; the production-parity stack enforces `read_only`, drops all Linux
capabilities, and enables `no-new-privileges`.

The uncompressed Docker image size budget is 100 MiB. Run
`just production-artifacts-check` to enforce the budget, labels, non-root user,
allowed filesystem contents, fail-closed hosted configuration, deterministic
web output, and secret-shaped artifact scan. The check prints the measured API
size and web artifact checksum.

## Forward-Only Migrations

SQLx migrations are embedded in the API binary; migration files are not copied
into the runtime image and no parallel schema mechanism exists. Local
`RAG_DEBUGGER_ENV=local` startup still applies pending migrations for the fast
developer workflow. Test, staging, and production startup never apply schema
changes: they verify every embedded migration is present with the expected
checksum before bootstrapping data or binding the listener.

For staging or production, use the exact API image digest selected for the
release and a dedicated migration credential:

```sh
docker run --rm \
  --env RAG_DEBUGGER_ENV=staging \
  --env DATABASE_URL \
  ghcr.io/muneebhoda/rag-debugger@sha256:<digest> migrate
```

The operator sequence is:

1. Select the immutable API digest and matching web checksum.
2. Verify the environment recovery point and acquire the migration identity.
3. Run `<api-image> migrate` once and require exit status zero.
4. Start the API with the lower-privilege runtime database identity.
5. Require `/readyz`, then run the synthetic canary before routing traffic.

Migration errors stop the sequence. Migrations are append-only and
forward-only; there is no automatic database rollback. Application rollback
requires a retained image compatible with the migrated schema. The runtime
identity does not need schema-write privileges, but it must be able to read
SQLx's `_sqlx_migrations` table so startup can verify version and checksum
parity.

## Web Artifact And Runtime Configuration

Build the application with the locked npm dependency graph:

```sh
npm --prefix apps/web ci
npm --prefix apps/web run build
```

`apps/web/dist` is the immutable application artifact. It loads
`/runtime-config.js` before the application module. `VITE_API_BASE_URL` remains
the local/test fallback only; staging and production must supply this public
runtime object beside the promoted artifact:

```js
window.CORPUSLAB = {
  apiBaseUrl: "https://api.staging.example.com",
  environment: "staging",
  releaseSha: "0123456789abcdef0123456789abcdef01234567",
};
```

Those are the only supported browser-visible runtime fields. The file must
never contain database URLs, passwords, provider tokens, CorpusLab API keys,
cookie values, corpus content, queries, traces, or reports. The deployment job
generates this separate public file from protected environment metadata,
validates the API origin and full commit SHA, and records its checksum beside
the unchanged application artifact checksum. Staging and production therefore
reuse the same compiled JS/CSS without source edits or environment rebuilds.

The production web image serves the same `dist` through unprivileged Nginx and
generates `/runtime-config.js` at container start from
`CORPUSLAB_PUBLIC_API_BASE_URL`, `CORPUSLAB_ENVIRONMENT`, and
`CORPUSLAB_RELEASE_SHA`. Its root filesystem is read-only; only a bounded
`/tmp` mount holds the generated public config and Nginx runtime files. Static
request logging is disabled so paths and query strings do not enter container
logs.

## Production-Parity Compose

The existing `docker compose up -d postgres` development flow is unchanged.
The opt-in stack uses the actual API and web images, Postgres 17, named storage,
readiness gates, and bounded resource defaults:

```sh
just production-up
# open http://127.0.0.1:15173/login
just production-down
```

Defaults are synthetic local-only credentials. Override
`CORPUSLAB_PARITY_DATABASE_PASSWORD`, `CORPUSLAB_PARITY_BOOTSTRAP_EMAIL`, and
`CORPUSLAB_PARITY_BOOTSTRAP_PASSWORD` in the ignored `.env` when needed; never
reuse them in a hosted environment. `migrate` must complete before the API
starts, `/readyz` must pass before the web service starts, and both application
containers run with read-only root filesystems.

Run the real packaged login → sample corpus → index → retrieve → trace → report
flow with:

```sh
just production-e2e
```

That command creates a fresh named database volume and removes it afterward.

## Shutdown And Current Limit

The API receives `SIGTERM` and Ctrl-C directly through its exec-form
entrypoint. Axum stops accepting new connections and waits for in-flight
requests to finish. The platform must allow at least 30 seconds between
`SIGTERM` and `SIGKILL`. CorpusLab does not impose an additional application
drain timeout yet; add one only if observed long-running requests exceed the
platform grace period.
