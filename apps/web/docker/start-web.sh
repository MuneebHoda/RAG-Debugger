#!/bin/sh
set -eu

: "${CORPUSLAB_PUBLIC_API_BASE_URL:?CORPUSLAB_PUBLIC_API_BASE_URL is required}"
: "${CORPUSLAB_ENVIRONMENT:?CORPUSLAB_ENVIRONMENT is required}"
: "${CORPUSLAB_RELEASE_SHA:?CORPUSLAB_RELEASE_SHA is required}"

if ! printf '%s' "$CORPUSLAB_PUBLIC_API_BASE_URL" | grep -Eq '^https?://[A-Za-z0-9._-]+(:[0-9]+)?$'; then
  echo "CORPUSLAB_PUBLIC_API_BASE_URL must be an HTTP(S) origin without a path" >&2
  exit 1
fi

case "$CORPUSLAB_ENVIRONMENT" in
  local|test|staging|production) ;;
  *) echo "CORPUSLAB_ENVIRONMENT must be local, test, staging, or production" >&2; exit 1 ;;
esac

if [ "${#CORPUSLAB_RELEASE_SHA}" -ne 40 ] || printf '%s' "$CORPUSLAB_RELEASE_SHA" | grep -Eq '[^0-9a-f]'; then
  echo "CORPUSLAB_RELEASE_SHA must be a full lowercase commit SHA" >&2
  exit 1
fi

render_runtime_config() {
  printf 'window.CORPUSLAB={apiBaseUrl:"%s",environment:"%s",releaseSha:"%s"};\n' \
    "$CORPUSLAB_PUBLIC_API_BASE_URL" \
    "$CORPUSLAB_ENVIRONMENT" \
    "$CORPUSLAB_RELEASE_SHA"
}

if [ "${1:-}" = "render-runtime-config" ]; then
  render_runtime_config
  exit 0
fi

render_runtime_config > /tmp/runtime-config.js

exec nginx -g 'daemon off;'
