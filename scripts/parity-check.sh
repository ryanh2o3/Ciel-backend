#!/usr/bin/env bash
# Cross-impl smoke / parity checks via X-Ciel-Backend header.
# Usage:
#   BASE_URL=http://localhost:8080 EMAIL=demo@example.com PASSWORD='ChangeMe123!' \
#     ./scripts/parity-check.sh
# Or against Traefik:
#   BASE_URL=https://home-api.ciel-social.eu ./scripts/parity-check.sh
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
EMAIL="${EMAIL:-demo@example.com}"
PASSWORD="${PASSWORD:-ChangeMe123!}"
BACKENDS="${BACKENDS:-rust spring dotnet}"

red() { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
fail=0

check() {
  local backend="$1"
  local name="$2"
  local method="$3"
  local path="$4"
  local data="${5:-}"
  local auth="${6:-}"

  local args=(-sS -D /tmp/ciel-parity-headers -o /tmp/ciel-parity-body -w '%{http_code}' \
    -X "$method" "${BASE_URL}${path}" \
    -H "X-Ciel-Backend: ${backend}" \
    -H "Content-Type: application/json")
  if [[ -n "$auth" ]]; then
    args+=(-H "Authorization: Bearer ${auth}")
  fi
  if [[ -n "$data" ]]; then
    args+=(-d "$data")
  fi

  local code
  code="$(curl "${args[@]}" || true)"
  local served
  served="$(grep -i '^x-ciel-served-by:' /tmp/ciel-parity-headers | awk '{print tolower($2)}' | tr -d '\r' || true)"

  if [[ "$code" != "200" && "$code" != "201" && "$code" != "202" && "$code" != "204" ]]; then
    red "[$backend] FAIL $name → HTTP $code body=$(head -c 200 /tmp/ciel-parity-body)"
    fail=1
    return 1
  fi
  if [[ -n "$served" && "$served" != "$backend" ]]; then
    red "[$backend] FAIL $name → expected X-Ciel-Served-By=$backend got=$served"
    fail=1
    return 1
  fi
  green "[$backend] OK $name (HTTP $code, served-by=${served:-n/a})"
  return 0
}

for backend in $BACKENDS; do
  echo "=== backend=$backend ==="
  check "$backend" health GET /health || continue

  code="$(curl -sS -D /tmp/ciel-parity-headers -o /tmp/ciel-parity-body -w '%{http_code}' \
    -X POST "${BASE_URL}/v1/auth/login" \
    -H "X-Ciel-Backend: ${backend}" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}" || true)"
  if [[ "$code" != "200" ]]; then
    red "[$backend] FAIL login → HTTP $code $(head -c 200 /tmp/ciel-parity-body)"
    fail=1
    continue
  fi
  access="$(python3 -c 'import json,sys; print(json.load(open("/tmp/ciel-parity-body"))["access_token"])' 2>/dev/null || true)"
  refresh="$(python3 -c 'import json,sys; print(json.load(open("/tmp/ciel-parity-body"))["refresh_token"])' 2>/dev/null || true)"
  if [[ -z "$access" ]]; then
    red "[$backend] FAIL login parse"
    fail=1
    continue
  fi
  green "[$backend] OK login"

  check "$backend" me GET /v1/auth/me "" "$access" || true
  check "$backend" feed GET "/v1/feed?limit=5" "" "$access" || true
  check "$backend" feed_refresh POST /v1/feed/refresh "" "$access" || true

  if [[ -n "$refresh" ]]; then
    check "$backend" refresh POST /v1/auth/refresh "{\"refresh_token\":\"${refresh}\"}" || true
  fi
done

if [[ "$fail" -ne 0 ]]; then
  red "parity-check: failures detected"
  exit 1
fi
green "parity-check: all exercised backends OK"
