#!/usr/bin/env bash
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

manifest=${1:-}
[[ -f "$manifest" ]] || { echo "usage: verify-published-release.sh <release-manifest.json>" >&2; exit 1; }
[[ "${GITHUB_REPOSITORY:-}" = "MuneebHoda/RAG-Debugger" ]] || { echo "published verification requires the canonical repository" >&2; exit 1; }

release_sha=${CORPUSLAB_RELEASE_SHA:-}
version=${CORPUSLAB_VERSION:-}
[[ "$release_sha" =~ ^[a-f0-9]{40}$ ]] || { echo "CORPUSLAB_RELEASE_SHA must independently identify the expected source commit" >&2; exit 1; }
[[ "$version" =~ ^0\.[0-9]+\.[0-9]+(-rc\.[1-9][0-9]*)?$ ]] || { echo "CORPUSLAB_VERSION must independently identify the expected application version" >&2; exit 1; }
[[ "$(jq -r '.source.commit' "$manifest")" = "$release_sha" ]] || { echo "release manifest source commit does not match the independently resolved source" >&2; exit 1; }
[[ "$(jq -r '.release.application_version' "$manifest")" = "$version" ]] || { echo "release manifest application version does not match the independently resolved version" >&2; exit 1; }
image=$(jq -r '.api.image' "$manifest")
digest=$(jq -r '.api.digest' "$manifest")
image_reference=$(jq -r '.api.reference' "$manifest")
artifact_directory=$(CDPATH= cd -- "$(dirname -- "$manifest")" && pwd)
web_artifact=$(jq -r '.web.artifact' "$manifest")
signer="github.com/MuneebHoda/RAG-Debugger/.github/workflows/publish-artifacts.yml"

node scripts/release-artifacts.mjs verify-manifest "$manifest" published

gh attestation verify "oci://$image_reference" --repo "$GITHUB_REPOSITORY" \
  --signer-workflow "$signer" --source-digest "$release_sha" >/dev/null
gh attestation verify "oci://$image_reference" --repo "$GITHUB_REPOSITORY" \
  --signer-workflow "$signer" --source-digest "$release_sha" \
  --predicate-type https://spdx.dev/Document/v2.3 >/dev/null
gh attestation verify "$artifact_directory/$web_artifact" --repo "$GITHUB_REPOSITORY" \
  --signer-workflow "$signer" --source-digest "$release_sha" >/dev/null
gh attestation verify "$artifact_directory/$web_artifact" --repo "$GITHUB_REPOSITORY" \
  --signer-workflow "$signer" --source-digest "$release_sha" \
  --predicate-type https://spdx.dev/Document/v2.3 >/dev/null

docker pull "$image_reference" >/dev/null
docker image inspect --format '{{json .RepoDigests}}' "$image_reference" |
  jq -e --arg expected "$image@$digest" 'index($expected) != null' >/dev/null ||
  { echo "pulled API image digest mismatch" >&2; exit 1; }
[[ "$(docker image inspect --format '{{.Architecture}}' "$image_reference")" = "amd64" ]] || { echo "published API image architecture mismatch" >&2; exit 1; }
[[ "$(docker image inspect --format '{{.Config.User}}' "$image_reference")" = "65532:65532" ]] || { echo "published API image user mismatch" >&2; exit 1; }
[[ "$(docker image inspect --format '{{index .Config.Labels "org.opencontainers.image.source"}}' "$image_reference")" = "https://github.com/MuneebHoda/RAG-Debugger" ]] || { echo "published API source label mismatch" >&2; exit 1; }
[[ "$(docker image inspect --format '{{index .Config.Labels "org.opencontainers.image.revision"}}' "$image_reference")" = "$release_sha" ]] || { echo "published API revision label mismatch" >&2; exit 1; }
[[ "$(docker image inspect --format '{{index .Config.Labels "org.opencontainers.image.version"}}' "$image_reference")" = "$version" ]] || { echo "published API version label mismatch" >&2; exit 1; }
[[ "$(docker image inspect --format '{{index .Config.Labels "org.opencontainers.image.licenses"}}' "$image_reference")" = "MIT" ]] || { echo "published API license label mismatch" >&2; exit 1; }

suffix=${GITHUB_RUN_ID:-$$}-${GITHUB_RUN_ATTEMPT:-1}
network="corpuslab-release-$suffix"
postgres_container="corpuslab-release-postgres-$suffix"
api_container="corpuslab-release-api-$suffix"

cleanup() {
  docker rm -f "$api_container" "$postgres_container" >/dev/null 2>&1 || true
  docker network rm "$network" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

docker network create "$network" >/dev/null
docker run -d --name "$postgres_container" --network "$network" --network-alias postgres \
  -e POSTGRES_DB=corpuslab -e POSTGRES_USER=corpuslab -e POSTGRES_PASSWORD=release-verification-only \
  postgres:17-alpine@sha256:18cfe3ef5e6815560c98237d6216d1e5119702fb0f3894c8785dd58b8bbe5d73 >/dev/null
for _ in $(seq 1 30); do
  docker exec "$postgres_container" pg_isready -U corpuslab -d corpuslab >/dev/null 2>&1 && break
  sleep 1
done
docker exec "$postgres_container" pg_isready -U corpuslab -d corpuslab >/dev/null

database_url=postgres://corpuslab:release-verification-only@postgres:5432/corpuslab
docker run --rm --network "$network" --read-only --cap-drop ALL --security-opt no-new-privileges \
  -e RAG_DEBUGGER_ENV=test -e "DATABASE_URL=$database_url" "$image_reference" migrate

docker run -d --name "$api_container" --network "$network" --read-only --cap-drop ALL \
  --security-opt no-new-privileges \
  -e RAG_DEBUGGER_ENV=test \
  -e RAG_DEBUGGER_RELEASE_SHA="$release_sha" \
  -e RAG_DEBUGGER_API_HOST=0.0.0.0 \
  -e RAG_DEBUGGER_API_PORT=8080 \
  -e RAG_DEBUGGER_WEB_ORIGIN=http://127.0.0.1:15173 \
  -e RAG_DEBUGGER_PUBLIC_API_BASE_URL=http://127.0.0.1:18080 \
  -e RAG_DEBUGGER_STORAGE_BACKEND=postgres \
  -e "DATABASE_URL=$database_url" \
  -e RAG_DEBUGGER_DEPLOYMENT_MODE=hosted \
  -e RAG_DEBUGGER_AUTH_PROVIDER=local \
  -e RAG_DEBUGGER_SESSION_COOKIE_NAME=corpuslab_test_session \
  -e RAG_DEBUGGER_SESSION_COOKIE_SECURE=false \
  -e RAG_DEBUGGER_BOOTSTRAP_EMAIL=release-verification@example.test \
  -e RAG_DEBUGGER_BOOTSTRAP_PASSWORD=release-verification-password \
  -e RAG_DEBUGGER_BOOTSTRAP_USER_NAME='Release Verification Owner' \
  -e RAG_DEBUGGER_BOOTSTRAP_ORGANIZATION='Release Verification Organization' \
  -e RAG_DEBUGGER_BOOTSTRAP_WORKSPACE='Release Verification Workspace' \
  -e RAG_DEBUGGER_EMBEDDING_PROVIDER=local \
  "$image_reference" >/dev/null
for _ in $(seq 1 30); do
  docker exec "$api_container" /usr/local/bin/rag-debugger-api readycheck >/dev/null 2>&1 && break
  sleep 1
done
docker exec "$api_container" /usr/local/bin/rag-debugger-api readycheck >/dev/null

echo "Verified published API image $image@$digest and $web_artifact."
