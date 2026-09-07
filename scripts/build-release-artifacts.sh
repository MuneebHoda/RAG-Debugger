#!/usr/bin/env bash
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

release_sha=${CORPUSLAB_RELEASE_SHA:-}
version=${CORPUSLAB_VERSION:-}
api_image=${CORPUSLAB_API_IMAGE:-}

[[ "$release_sha" =~ ^[a-f0-9]{40}$ ]] || { echo "CORPUSLAB_RELEASE_SHA must be a full lowercase commit SHA" >&2; exit 1; }
[[ "$version" =~ ^0\.[0-9]+\.[0-9]+(-rc\.[1-9][0-9]*)?$ ]] || { echo "CORPUSLAB_VERSION does not match the release policy" >&2; exit 1; }
[[ -n "$api_image" && "$api_image" != *"@"* && "$api_image" != *":latest" ]] || { echo "CORPUSLAB_API_IMAGE is invalid" >&2; exit 1; }
[[ "$(git rev-parse HEAD)" = "$release_sha" ]] || { echo "checked-out source does not match CORPUSLAB_RELEASE_SHA" >&2; exit 1; }
[[ -s Cargo.lock && -s apps/web/package-lock.json ]] || { echo "required lockfile is missing or empty" >&2; exit 1; }

package_versions=$(cargo metadata --locked --no-deps --format-version 1 | jq -r '[.packages[].version] | unique | join("\n")')
[[ "$package_versions" = "$version" ]] || { echo "workspace package versions do not match CORPUSLAB_VERSION" >&2; exit 1; }
[[ "$(node -p "require('./apps/web/package.json').version")" = "$version" ]] || { echo "web package version does not match CORPUSLAB_VERSION" >&2; exit 1; }

export SOURCE_DATE_EPOCH
SOURCE_DATE_EPOCH=$(git show -s --format=%ct "$release_sha")

docker buildx build --platform linux/amd64 --load --provenance=false \
  --build-arg "OCI_REVISION=$release_sha" \
  --build-arg "OCI_VERSION=$version" \
  --tag "$api_image:$release_sha" .

npm --prefix apps/web ci
npm --prefix apps/web audit --omit=dev --audit-level=high
VITE_API_BASE_URL=http://127.0.0.1:8080 npm --prefix apps/web run build
node scripts/release-artifacts.mjs prepare

archive=$(jq -r '.web.artifact' target/release-artifacts/build-identities.json)
(
  cd target/release-artifacts/web-application
  find . -type f -print | LC_ALL=C sort | TZ=UTC zip -X -q "../$archive" -@
)
node scripts/release-artifacts.mjs finalize

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    printf 'api_tag=%s\n' "$api_image:$release_sha"
    jq -r '"web_artifact=\(.web.artifact)", "web_application_sha256=\(.web.application_sha256)", "web_archive_sha256=\(.web.archive_sha256)", "migration_sha256=\(.migrations.sha256)"' target/release-artifacts/build-identities.json
  } >> "$GITHUB_OUTPUT"
fi
