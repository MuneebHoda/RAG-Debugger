# Trusted Artifact Publication

CorpusLab publishes deployment inputs without deploying them. This workflow
implements the build-and-release boundary approved in
[ADR 0010](adr/0010-private-alpha-deployment.md) and preserves the application
identity defined by [Production Artifacts](production-artifacts.md).

## Trust And Trigger Model

`.github/workflows/publish-artifacts.yml` has two trusted entry points:

- a `push` of an exact commit to the canonical repository's protected `main`,
  with publication held until that same SHA's required checks succeed; and
- publication of a GitHub Release whose existing tag matches the repository's
  reviewed `v0.x.y` or `v0.x.y-rc.n` policy, resolves to a `main` commit, and
  matches that commit's application version.

Pull requests, forks, comments, issue/label events, manual workflow dispatch,
and automation branches cannot invoke a publishing job. The main path checks
the exact source SHA and requires successful Rust, Web, database migration,
documentation, production-artifact, release-dry-run, Cargo Deny, Rust/Web
coverage-upload, and CodeQL analysis checks for that SHA. A successful CodeQL
check-run is not sufficient: the gate independently resolves the merged pull
request associated with the exact main SHA and queries its open code-scanning
alerts. Any open finding, or an API/query failure, blocks publication.
Dependency Review is the protected-branch pre-merge gate; it does not run on
main. The active main ruleset has no bypass actor, so a commit cannot become
trusted main state until that review and the other required checks pass.

Every external Action reference is a reviewed full commit SHA. Trusted-main
checkout uses `github.sha` directly. Read-only release verification starts from
the constant `main` ref and detaches to the independently resolved commit before
executing repository code. Every checkout sets `persist-credentials: false`.
The workflow default is no token permissions:

| Job                      | Permissions                                                                                               | Purpose                                                                                      |
| ------------------------ | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Main gate                | `checks: read`, `contents: read`, `pull-requests: read`, `security-events: read`                          | Validate trusted source, completed checks, and the exact commit's open CodeQL alerts         |
| Main publication         | `contents: read`, `packages: write`, `id-token: write`, `attestations: write`, `artifact-metadata: write` | Push GHCR, create GitHub attestations, retain the release bundle                             |
| Release resolver         | `contents: read`                                                                                          | Resolve and validate an existing annotated tag without executing its source                  |
| Read-only release verify | `actions: read`, `attestations: read`, `contents: read`, `packages: read`                                 | Bind the resolved source/version to one successful main run and verify its bundle/digests    |
| Version alias mutation   | `actions: read`, `contents: write`, `packages: write`                                                     | Download that exact verified bundle, add its same-digest alias, and attach its release files |

The GHCR token is introduced only after the final artifacts have been built and
scanned. Build steps receive no staging, production, database, Cloudflare,
Render, or customer secret. No GitHub Environment is used by publication.
The write-capable version-alias job never checks out or executes repository
source. It receives the independently resolved tag identity plus the verified
publication run ID and manifest checksum, downloads that exact immutable
bundle again, and checks the manifest checksum/source/version before its first
package or release mutation.

## Build And Artifact Identities

One invocation of `scripts/build-release-artifacts.sh` builds one Linux AMD64
API image and one Vite application from the exact commit and locked Cargo/npm
graphs. There are no environment-specific application builds.

- **API:** `ghcr.io/muneebhoda/rag-debugger:<full-commit-sha>` is pushed and
  the registry-returned `sha256:` digest is recorded. Deployment must use
  `ghcr.io/muneebhoda/rag-debugger@sha256:<digest>`, never the tag. An approved
  semantic version may become a second informational tag for the same digest;
  the job refuses to move an existing version tag and never creates `latest`.
- **Web application:** `corpuslab-web-<full-commit-sha>.zip` is created with
  sorted entries, fixed metadata, and no `runtime-config.js`. The application
  checksum is the SHA-256 of `web-application.files.sha256`, using the same
  per-file algorithm as #103. The ZIP transport checksum is recorded
  separately. Staging and production later add their public runtime config and
  its separate checksum without changing either application identity.
- **Migrations:** `migrations.files.sha256` contains sorted per-file SHA-256
  values for the exact `migrations/*.sql` files embedded by SQLx into the API
  binary. Its own SHA-256 is the migration-set identity. Deployment therefore
  binds source → API digest → embedded SQLx set → web application checksum
  without introducing another migration mechanism.

The image retains OCI source, revision, version, and MIT license labels. The
version must match every workspace package and `apps/web/package.json`.

## SBOM And Scan Policy

The workflow uses Syft v1.51.1 through the commit-pinned Anchore SBOM Action to
generate SPDX 2.3 JSON for the final local API image and the exact staged web
application files. File metadata cataloging is enabled so deployed files and
their digests appear in the SBOMs. Each SBOM is checksummed, included in the
release bundle, and bound to its API or web subject by a GitHub SBOM
attestation.

Trivy v0.74.0 scans the final API image and staged web filesystem for known
vulnerabilities and secret/private-key/credential patterns. Any known High or
Critical vulnerability, regardless of fix availability, or any secret finding
fails before registry authentication. `npm audit --omit=dev --audit-level=high`
also checks the locked production browser dependency graph used by the build.
There is no implicit allowlist. A future exception must be narrow, documented
with advisory, affected artifact, owner, expiry, and justification, and covered
by a reviewed policy test. Successful JSON scan reports contain no forbidden
finding and are checksummed in the bundle; findings are summarized by safe
identifier/category rather than printing matched content.

## GitHub Attestations

The trusted main job uses the GitHub-maintained `actions/attest` action and
short-lived GitHub OIDC identity to create:

1. SLSA provenance for the registry-returned API digest;
2. an SPDX SBOM attestation for that digest, also stored with the GHCR subject;
3. SLSA provenance for the web ZIP checksum; and
4. an SPDX SBOM attestation for the same web ZIP.

Before the workflow succeeds, GitHub CLI verification requires the canonical
repository, this workflow as signer, and the exact source commit. The image is
then pulled by digest, inspected, migrated against fresh Postgres through the
packaged explicit command, started read-only, and required to become ready. A
missing, invalid, or mismatched provenance/SBOM attestation stops the run.

Operators can repeat verification after authenticating to GHCR:

```sh
gh attestation verify \
  oci://ghcr.io/muneebhoda/rag-debugger@sha256:<digest> \
  --repo MuneebHoda/RAG-Debugger \
  --signer-workflow github.com/MuneebHoda/RAG-Debugger/.github/workflows/publish-artifacts.yml \
  --source-digest <full-commit-sha>

gh attestation verify corpuslab-web-<full-commit-sha>.zip \
  --repo MuneebHoda/RAG-Debugger \
  --signer-workflow github.com/MuneebHoda/RAG-Debugger/.github/workflows/publish-artifacts.yml \
  --source-digest <full-commit-sha>
```

Add `--predicate-type https://spdx.dev/Document/v2.3` to verify each SBOM
attestation.

## Release Manifest

`release-manifest.json` is JSON with schema version `1`. It is generated only
after registry digest resolution, scans, SBOM generation, and attestation
creation. The verifier independently recalculates every local checksum, opens
the ZIP without executing it, rejects unsafe paths and runtime config, compares
migrations to the exact checkout, validates SPDX/Trivy schemas, pulls the API
by digest, and verifies OCI labels/readiness.

Published verification requires an expected source SHA and application version
from outside the manifest. For a semantic release these values come from the
annotated-tag resolver, and the selected successful publication run must report
that exact SHA. A manifest cannot self-assert either identity; source or version
mismatch stops before an image alias or GitHub Release asset can be changed.

The manifest records this shape; placeholders below are descriptive, not
published identities:

```json
{
  "schema_version": 1,
  "publication": { "mode": "published" },
  "source": {
    "repository": "https://github.com/MuneebHoda/RAG-Debugger",
    "commit": "<full-commit-sha>"
  },
  "release": { "application_version": "0.x.y", "version_tag": null },
  "api": {
    "image": "ghcr.io/muneebhoda/rag-debugger",
    "digest": "sha256:<registry-digest>",
    "reference": "ghcr.io/muneebhoda/rag-debugger@sha256:<registry-digest>",
    "full_commit_tag": "ghcr.io/muneebhoda/rag-debugger:<full-commit-sha>"
  },
  "web": {
    "artifact": "corpuslab-web-<full-commit-sha>.zip",
    "application_sha256": "<application-checksum>",
    "archive_sha256": "<zip-checksum>",
    "runtime_config_identity": "excluded; generated and checksummed at deployment time"
  },
  "migrations": {
    "manifest": "migrations.files.sha256",
    "sha256": "<migration-set-checksum>"
  },
  "sboms": {
    "format": "SPDX-2.3",
    "api": {
      "artifact": "api.spdx.json",
      "sha256": "<checksum>",
      "subject": "<API digest>",
      "attestation": { "id": "<id>", "url": "<URL>" }
    },
    "web": {
      "artifact": "web.spdx.json",
      "sha256": "<checksum>",
      "subject": "<web ZIP checksum>",
      "attestation": { "id": "<id>", "url": "<URL>" }
    }
  },
  "provenance": {
    "api": {
      "subject": "<API digest>",
      "attestation": { "id": "<id>", "url": "<URL>" }
    },
    "web": {
      "subject": "<web ZIP checksum>",
      "attestation": { "id": "<id>", "url": "<URL>" }
    }
  },
  "scans": { "policy": "<fail-closed policy>", "api": {}, "web": {} },
  "workflow": {
    "name": "Publish deployment artifacts",
    "run_id": "<id>",
    "run_attempt": 1,
    "url": "<URL>"
  }
}
```

Build timestamps are omitted, so they cannot redefine artifact identity. The
manifest rejects database URLs, credentials, customer data, or runtime config
values.

## Pull-Request Dry Run

The `Release dry run` CI job has only `contents: read`. It checks out the exact
PR SHA, verifies lockfiles, builds the same API/web inputs once, proves the web
ZIP is deterministic, generates and validates both SPDX files, runs Trivy and
the production npm advisory gate, and validates a manifest explicitly marked
`dry-run`. Negative fixtures prove that unsafe triggers/permissions, checkout
in the write-capable alias job, Action tags, mutable selectors, missing
locks/attestations, source/version mismatches, checksum mismatches,
secret-shaped files, High/Critical results, and CodeQL alert-query failures
fail.

The dry run does not authenticate to GHCR, push packages, mint OIDC tokens,
create attestations, attach release assets, or deploy. GHCR digest resolution,
GitHub signing, pull-by-digest, and registry-backed verification execute only
after merge on the trusted-main path.

## Retention And Rollback

- Successful trusted-main bundles are retained as GitHub Actions artifacts for
  30 days. Failed or incomplete publication runs are not eligible deployment
  inputs even if an intermediate package exists.
- Publishing a reviewed semantic GitHub Release reverifies the exact main
  bundle, adds a same-digest GHCR version alias, and attaches the web ZIP,
  checksums, migration manifest, SPDX SBOMs, scan reports, and release manifest
  to that release. Release assets remain while the release is supported,
  deployed, or designated last-known-good.
- GHCR has no automatic deletion rule in this workflow. Package cleanup must
  retain every deployed digest and at least one compatible last-known-good
  digest. Remove a digest only after checking deployment/release records and
  replacement rollback coverage.
- Operators find rollback inputs in the last successful publication manifest
  or supported GitHub Release, then select the recorded API `@sha256:` reference
  and matching web application checksum. A mutable tag is never the selector.
- Application rollback redeploys retained application artifacts only. It never
  reverses SQLx migrations automatically; schema compatibility and any
  forward-fix migration remain an explicit operator decision.

This workflow publishes trust metadata and artifacts only. It does not create
provider credentials, provision Cloudflare/Render, or deploy staging or
production; those remain later issues.
