#!/usr/bin/env sh
set -eu

: "${CORPUSLAB_API_URL:?Set CORPUSLAB_API_URL, for example http://127.0.0.1:8080}"
: "${CORPUSLAB_API_KEY:?Create a trace-ingestion API key in Settings}"
: "${CORPUSLAB_PROJECT_ID:?Copy the project ID from Settings > API keys}"

if ! printf '%s\n' "${CORPUSLAB_PROJECT_ID}" |
  grep -Eq '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'; then
  echo "CORPUSLAB_PROJECT_ID must be a UUID." >&2
  exit 1
fi

header_file=$(mktemp "${TMPDIR:-/tmp}/corpuslab-ingest.XXXXXX")
trap 'rm -f "${header_file}"' EXIT HUP INT TERM
chmod 600 "${header_file}"
printf 'Authorization: Bearer %s\n' "${CORPUSLAB_API_KEY}" >"${header_file}"

curl --fail-with-body \
  --connect-timeout 5 \
  --max-time 30 \
  --request POST \
  --header "@${header_file}" \
  --header "Content-Type: application/json" \
  --data "{
    \"schema_version\": \"1\",
    \"project_id\": \"${CORPUSLAB_PROJECT_ID}\",
    \"external_trace_id\": \"native-demo-001\",
    \"privacy_mode\": \"snippets_allowed\",
    \"retrieval_mode\": \"hybrid\",
    \"top_k\": 1,
    \"retrieved_evidence\": [{
      \"external_chunk_id\": \"policy-chunk-7\",
      \"rank\": 1,
      \"score\": 0.31,
      \"lexical_score\": 0.21,
      \"semantic_score\": 0.38,
      \"citation_label\": \"E1\",
      \"snippet\": \"The index is published only after validation.\"
    }],
    \"failure_labels\": [\"weak_evidence\"],
    \"status\": \"failed\",
    \"latency_ms\": 42
  }" \
  "${CORPUSLAB_API_URL%/}/api/v1/traces/ingest"
