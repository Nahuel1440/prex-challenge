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

# expect_movement <status> <body fragment> <path> <json body>
expect_movement() {
  local expected_status="$1" expected_fragment="$2" path="$3" body="$4"
  expect_status "$expected_status" "$expected_fragment" \
    --request POST --header 'Content-Type: application/json' --data "$body" "$BASE_URL$path"
}

# Registers a client and prints its id, read from {"client_id":N}.
register_client_for_movements() {
  local response
  response="$(curl --silent --request POST --header 'Content-Type: application/json' \
    --data "$(new_client_body 'Movement Client' "MOVE$(date +%s)${RANDOM}")" "$BASE_URL/new_client")"
  response="${response#*:}"
  echo "${response%\}*}"
}

expect_status 200 '"status":"ok"' "$BASE_URL/health"

set_new_client_request "$(new_client_body 'Ana Perez' "$DOCUMENT_NUMBER")"
expect_status 201 '"client_id"' "${curl_args[@]}"

set_new_client_request "$(new_client_body 'Other Person' "$DOCUMENT_NUMBER")"
expect_status 409 'DOCUMENT_ALREADY_REGISTERED' "${curl_args[@]}"

set_new_client_request "$(new_client_body '   ' '123')"
expect_status 400 'INVALID_REQUEST' "${curl_args[@]}"

CLIENT_ID="$(register_client_for_movements)"
expect_movement 200 '"balance":100.50' /new_credit_transaction "{\"client_id\":$CLIENT_ID,\"credit_amount\":100.50}"
expect_movement 200 '"balance":70.50' /new_debit_transaction "{\"client_id\":$CLIENT_ID,\"debit_amount\":30}"
expect_movement 200 '"balance":-929.50' /new_debit_transaction "{\"client_id\":$CLIENT_ID,\"debit_amount\":1000}"
expect_movement 400 'INVALID_AMOUNT' /new_credit_transaction "{\"client_id\":$CLIENT_ID,\"credit_amount\":0}"
expect_movement 400 'INVALID_REQUEST' /new_credit_transaction "{\"client_id\":$CLIENT_ID,\"credit_amount\":\"100.50\"}"
expect_movement 404 'CLIENT_NOT_FOUND' /new_credit_transaction '{"client_id":999999999,"credit_amount":10}'

# The client above is Movement Client with a balance of -929.50 after the movements.
expect_status 200 "\"client_id\":$CLIENT_ID,\"client_name\":\"Movement Client\",\"birth_date\":\"1990-05-17\"" "$BASE_URL/client_balance?user_id=$CLIENT_ID"
expect_status 200 '"country":"AR","balance":-929.50}' "$BASE_URL/client_balance?user_id=$CLIENT_ID"
expect_status 404 'CLIENT_NOT_FOUND' "$BASE_URL/client_balance?user_id=999999999"
expect_status 400 'INVALID_REQUEST' "$BASE_URL/client_balance?user_id=abc"

echo "smoke passed"
