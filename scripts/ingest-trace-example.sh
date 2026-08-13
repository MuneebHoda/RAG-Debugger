#!/usr/bin/env sh
set -eu

: "${CORPUSLAB_API_URL:?Set CORPUSLAB_API_URL, for example http://127.0.0.1:8080}"
: "${CORPUSLAB_API_KEY:?Create a trace-ingestion API key in Settings}"
: "${CORPUSLAB_PROJECT_ID:?Copy the project ID from Settings > API keys}"

curl --fail-with-body \
  --request POST \
  --header "Authorization: Bearer ${CORPUSLAB_API_KEY}" \
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
