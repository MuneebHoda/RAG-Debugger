#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

release_sha=${CORPUSLAB_RELEASE_SHA:-$(git rev-parse HEAD)}
version=${CORPUSLAB_VERSION:-0.1.0}
api_image="corpuslab-api:verify-$release_sha"
web_image="corpuslab-web:verify-$release_sha"
work=$(mktemp -d)
container=

cleanup() {
  [ -z "$container" ] || docker rm "$container" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT INT TERM

docker buildx build --platform linux/amd64 --load --provenance=false \
  --build-arg "OCI_REVISION=$release_sha" --build-arg "OCI_VERSION=$version" \
  --tag "$api_image" .

user=$(docker image inspect --format '{{.Config.User}}' "$api_image")
[ "$user" = "65532:65532" ] || { echo "API image user is $user, expected 65532:65532" >&2; exit 1; }
architecture=$(docker image inspect --format '{{.Architecture}}' "$api_image")
[ "$architecture" = "amd64" ] || { echo "API image architecture is $architecture, expected amd64" >&2; exit 1; }

check_label() {
  label=$1
  expected=$2
  value=$(docker image inspect --format "{{index .Config.Labels \"$label\"}}" "$api_image")
  [ "$value" = "$expected" ] || { echo "OCI label $label is $value, expected $expected" >&2; exit 1; }
}
check_label org.opencontainers.image.source https://github.com/MuneebHoda/RAG-Debugger
check_label org.opencontainers.image.revision "$release_sha"
check_label org.opencontainers.image.version "$version"
check_label org.opencontainers.image.licenses MIT

container=$(docker create --platform linux/amd64 "$api_image")
docker export "$container" > "$work/api-rootfs.tar"
tar -tf "$work/api-rootfs.tar" > "$work/api-rootfs.txt"
grep -v '/$' "$work/api-rootfs.txt" | LC_ALL=C sort > "$work/api-files.txt"
printf '%s\n' \
  .dockerenv \
  dev/console \
  etc/hostname \
  etc/hosts \
  etc/mtab \
  etc/resolv.conf \
  etc/ssl/certs/ca-certificates.crt \
  usr/local/bin/rag-debugger-api > "$work/expected-api-files.txt"
diff -u "$work/expected-api-files.txt" "$work/api-files.txt"

size=$(docker image inspect --format '{{.Size}}' "$api_image")
budget=$((100 * 1024 * 1024))
[ "$size" -le "$budget" ] || { echo "API image size $size exceeds $budget bytes" >&2; exit 1; }

if docker run --rm --platform linux/amd64 -e RAG_DEBUGGER_ENV=production "$api_image" >/dev/null 2>&1; then
  echo "invalid hosted configuration unexpectedly started" >&2
  exit 1
fi

VITE_API_BASE_URL=http://127.0.0.1:8080 npm --prefix apps/web run build
hash_tree() {
  find apps/web/dist -type f | LC_ALL=C sort | while IFS= read -r file; do
    if command -v sha256sum >/dev/null 2>&1; then
      digest=$(sha256sum "$file" | cut -d ' ' -f 1)
    else
      digest=$(shasum -a 256 "$file" | cut -d ' ' -f 1)
    fi
    printf '%s  %s\n' "$digest" "${file#apps/web/dist/}"
  done
}
hash_tree > "$work/web-first.sha256"
VITE_API_BASE_URL=http://127.0.0.1:8080 npm --prefix apps/web run build
hash_tree > "$work/web-second.sha256"
diff -u "$work/web-first.sha256" "$work/web-second.sha256"

if grep -REn 'postgres(ql)?://|DATABASE_URL=|RAG_DEBUGGER_BOOTSTRAP_PASSWORD=|BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY' apps/web/dist; then
  echo "secret-shaped content found in web artifact" >&2
  exit 1
fi

docker buildx build --platform linux/amd64 --load --provenance=false \
  --build-arg "OCI_REVISION=$release_sha" --build-arg "OCI_VERSION=$version" \
  --tag "$web_image" --file apps/web/Dockerfile .

artifact_checksum=$(if command -v sha256sum >/dev/null 2>&1; then sha256sum "$work/web-second.sha256" | cut -d ' ' -f 1; else shasum -a 256 "$work/web-second.sha256" | cut -d ' ' -f 1; fi)
echo "API_IMAGE_SIZE_BYTES=$size"
echo "WEB_ARTIFACT_CHECKSUM=$artifact_checksum"
