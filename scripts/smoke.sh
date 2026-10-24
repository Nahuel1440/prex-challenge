#!/usr/bin/env bash
# Hits every endpoint that exists so far and fails on an unexpected status or body.
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
DOCUMENT_NUMBER="SMOKE$(date +%s)${RANDOM}"

# expect_status <expected_status> <expected_body_fragment> <curl args...>
expect_status() {
  local expected_status="$1" expected_fragment="$2"
  shift 2
  local response status body
  response="$(curl --silent --write-out '\n%{http_code}' "$@")"
  status="${response##*$'\n'}"
  body="${response%$'\n'*}"

  if [[ "$status" != "$expected_status" || "$body" != *"$expected_fragment"* ]]; then
    echo "FAIL: expected $expected_status containing '$expected_fragment', got $status: $body" >&2
    exit 1
  fi
  echo "ok: $expected_status $expected_fragment"
}

new_client_body() {
  printf '{"client_name":"%s","birth_date":"1990-05-17","document_number":"%s","country":"AR"}' "$1" "$2"
}

set_new_client_request() {
  curl_args=(--request POST --header 'Content-Type: application/json' --data "$1" "$BASE_URL/new_client")
}

expect_status 200 '"status":"ok"' "$BASE_URL/health"

set_new_client_request "$(new_client_body 'Ana Perez' "$DOCUMENT_NUMBER")"
expect_status 201 '"client_id"' "${curl_args[@]}"

set_new_client_request "$(new_client_body 'Other Person' "$DOCUMENT_NUMBER")"
expect_status 409 'DOCUMENT_ALREADY_REGISTERED' "${curl_args[@]}"

set_new_client_request "$(new_client_body '   ' '123')"
expect_status 400 'INVALID_REQUEST' "${curl_args[@]}"

echo "smoke passed"
