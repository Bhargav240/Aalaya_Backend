#!/usr/bin/env bash
set -euo pipefail

readonly database_url="${MIGRATION_DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:54322/postgres}"
readonly api_root="http://127.0.0.1:18080"
readonly api_base="$api_root/api/v1"
readonly platform_org="00000000-0000-0000-0000-000000000001"
readonly server_log="$(mktemp -t hamsa-email-e2e.XXXXXX)"
server_pid=""
current_step="startup"

cleanup() {
  local exit_code=$?
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -f "$server_log"
  if [[ $exit_code -ne 0 ]]; then
    echo "FAILED: $current_step" >&2
  fi
}
trap cleanup EXIT

need() {
  command -v "$1" >/dev/null || {
    echo "Required command is missing: $1" >&2
    exit 1
  }
}
need cargo
need curl
need jq
need psql

psql "$database_url" -v ON_ERROR_STOP=1 -q <<SQL
INSERT INTO public.organizations (id,name)
VALUES ('$platform_org','Hamsa Explore Platform')
ON CONFLICT (id) DO NOTHING;
SQL

env \
  PROFILE=development \
  DATABASE_URL="$database_url" \
  MIGRATION_DATABASE_URL="$database_url" \
  BIND_ADDR=127.0.0.1:18080 \
  LOG_FORMAT=json \
  RUST_LOG=info \
  JWT_SECRET=hamsa-local-e2e-jwt-secret-not-for-production \
  PROVISIONING_KEY=hamsa-local-e2e-provisioning-key \
  PLATFORM_ORG_ID="$platform_org" \
  EMAIL_PROVIDER=log \
  DEBUG_EMAIL_OTP=true \
  OTP_RESEND_COOLDOWN_SECS=0 \
  EMAIL_OTP_MAX_SENDS=20 \
  ONBOARDING_LAYOUT_PATH=./config/onboarding-layout.example.yaml \
  VERIFICATION_METHOD=mock \
  cargo run -p pg-server >"$server_log" 2>&1 &
server_pid=$!

for _ in {1..100}; do
  if curl -fsS "$api_root/health" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "E2E backend exited before becoming ready" >&2
    exit 1
  fi
  sleep 0.1
done
curl -fsS "$api_root/health" | jq -e '.status == "ok" and .db_ready == true' >/dev/null

latest_otp() {
  local code=""
  for _ in {1..100}; do
    code="$(sed -n 's/.*"otp_code":"\([0-9][0-9][0-9][0-9][0-9][0-9]\)".*/\1/p' "$server_log" | tail -n 1)"
    if [[ -n "$code" ]]; then
      printf '%s' "$code"
      return
    fi
    sleep 0.05
  done
  echo "Development OTP was not emitted" >&2
  exit 1
}

login() {
  local email="$1"
  curl -fsS -X POST "$api_base/auth/email/send-otp" \
    -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg email "$email" '{email:$email}')" >/dev/null
  local code
  code="$(latest_otp)"
  curl -fsS -X POST "$api_base/auth/email/verify-otp" \
    -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg email "$email" --arg code "$code" '{email:$email,code:$code}')"
}

bearer_get() {
  local token="$1"
  local path="$2"
  curl -fsS "$api_base$path" -H "Authorization: Bearer $token"
}

readonly run_id="$(date +%s)-$$"
readonly owner_email="owner-$run_id@example.com"
readonly tenant_email="tenant-$run_id@example.com"
readonly operator_email="operator-$run_id@example.com"

current_step="provision organization"
provisioned="$(curl -fsS -X POST "$api_base/orgs" \
  -H 'Content-Type: application/json' \
  -H 'x-provisioning-key: hamsa-local-e2e-provisioning-key' \
  -d "$(jq -nc --arg name "E2E PG $run_id" --arg email "$owner_email" \
    '{name:$name,owner_email:$email,owner_name:"E2E Owner"}')")"
org_id="$(jq -er '.org_id' <<<"$provisioned")"

current_step="owner email login"
owner_login="$(login "$owner_email")"
owner_access="$(jq -er '.access_token' <<<"$owner_login")"

branch_id="$(bearer_get "$owner_access" /branches | jq -er '.[0].id')"
room_id="$(bearer_get "$owner_access" "/branches/$branch_id/rooms" | jq -er '.[0].id')"
bed_id="$(bearer_get "$owner_access" "/rooms/$room_id/beds" | jq -er '.[0].id')"

current_step="platform operator fixture"
operator_id="$(cat /proc/sys/kernel/random/uuid 2>/dev/null || uuidgen | tr '[:upper:]' '[:lower:]')"
psql "$database_url" -v ON_ERROR_STOP=1 -q <<SQL
INSERT INTO public.platform_users (id,org_id,email,full_name,status)
VALUES ('$operator_id','$platform_org','$operator_email','E2E Operator','active');
INSERT INTO public.user_org_roles (user_id,org_id,role,membership_status)
VALUES ('$operator_id','$platform_org','platform_operator','active');
SQL
current_step="operator email login"
operator_login="$(login "$operator_email")"
operator_access="$(jq -er '.access_token' <<<"$operator_login")"

current_step="publish listing"
curl -fsS -X PUT "$api_base/listings/$branch_id" \
  -H "Authorization: Bearer $owner_access" \
  -H 'Content-Type: application/json' \
  -d '{"title":"E2E Main Branch","city":"Bengaluru","area":"Test Area","description":"End-to-end test listing"}' >/dev/null
curl -fsS -X POST "$api_base/listings/$branch_id/submit" \
  -H "Authorization: Bearer $owner_access" >/dev/null
curl -fsS -X POST "$api_base/operator/listings/$branch_id/publish" \
  -H "Authorization: Bearer $operator_access" >/dev/null

current_step="tenant prospect email login"
tenant_login="$(login "$tenant_email")"
tenant_prospect_access="$(jq -er '.access_token' <<<"$tenant_login")"
tenant_refresh="$(jq -er '.refresh_token' <<<"$tenant_login")"
bearer_get "$tenant_prospect_access" /discover/listings \
  | jq -e --arg branch "$branch_id" 'any(.[]; .branch_id == $branch)' >/dev/null
curl -fsS -X POST "$api_base/listings/$branch_id/enquire" \
  -H "Authorization: Bearer $tenant_prospect_access" \
  -H 'Content-Type: application/json' \
  -d '{"note":"E2E enquiry"}' | jq -e '.status == "received"' >/dev/null

current_step="create tenant invitation"
invitation="$(curl -fsS -X POST "$api_base/tenant-invitations" \
  -H "Authorization: Bearer $owner_access" \
  -H 'Content-Type: application/json' \
  -d "$(jq -nc \
    --arg email "$tenant_email" --arg branch "$branch_id" --arg room "$room_id" --arg bed "$bed_id" \
    '{email:$email,full_name:"E2E Tenant",branch_id:$branch,intended_room_id:$room,
      intended_bed_id:$bed,monthly_rent_paise:900000,deposit_paise:1800000,
      planned_move_in_date:"2026-09-01",require_identity_verification:true}')")"
invitation_id="$(jq -er '.id' <<<"$invitation")"

current_step="accept invitation"
bearer_get "$tenant_prospect_access" /me/tenant-invitations \
  | jq -e --arg id "$invitation_id" 'any(.[]; .id == $id)' >/dev/null
curl -fsS -X POST "$api_base/tenant-invitations/$invitation_id/accept" \
  -H "Authorization: Bearer $tenant_prospect_access" \
  | jq -e '.status == "kyc_pending"' >/dev/null
current_step="manual KYC review"
curl -fsS -X POST "$api_base/tenant-invitations/$invitation_id/manual-review" \
  -H "Authorization: Bearer $owner_access" \
  -H 'Content-Type: application/json' \
  -d '{"decision":"approved","reason":"E2E manual identity verification"}' \
  | jq -e '.status == "approval_pending"' >/dev/null
current_step="atomic tenant activation"
curl -fsS -X POST "$api_base/tenant-invitations/$invitation_id/approve" \
  -H "Authorization: Bearer $owner_access" \
  | jq -e '.status == "active"' >/dev/null

current_step="list membership contexts"
memberships="$(bearer_get "$tenant_prospect_access" /me/memberships)"
jq -e --arg org "$platform_org" 'any(.[]; .organization_id == $org and .role == "prospect")' \
  <<<"$memberships" >/dev/null
jq -e --arg org "$org_id" 'any(.[]; .organization_id == $org and .role == "tenant")' \
  <<<"$memberships" >/dev/null

current_step="switch to tenant context"
tenant_context="$(curl -fsS -X POST "$api_base/auth/switch-organization" \
  -H "Authorization: Bearer $tenant_prospect_access" \
  -H 'Content-Type: application/json' \
  -d "$(jq -nc --arg org "$org_id" --arg refresh "$tenant_refresh" \
    '{organization_id:$org,role:"tenant",refresh_token:$refresh}')")"
tenant_access="$(jq -er '.access_token' <<<"$tenant_context")"
tenant_refresh="$(jq -er '.refresh_token' <<<"$tenant_context")"
bearer_get "$tenant_access" /me/ledger | jq -e '.status == "active"' >/dev/null
bearer_get "$tenant_access" /discover/listings | jq -e 'length >= 1' >/dev/null

tenant_branch_status="$(curl -sS -o /dev/null -w '%{http_code}' "$api_base/branches" \
  -H "Authorization: Bearer $tenant_access")"
[[ "$tenant_branch_status" == "403" ]]

current_step="switch back to prospect context"
prospect_context="$(curl -fsS -X POST "$api_base/auth/switch-organization" \
  -H "Authorization: Bearer $tenant_access" \
  -H 'Content-Type: application/json' \
  -d "$(jq -nc --arg org "$platform_org" --arg refresh "$tenant_refresh" \
    '{organization_id:$org,role:"prospect",refresh_token:$refresh}')")"
prospect_access="$(jq -er '.access_token' <<<"$prospect_context")"
bearer_get "$prospect_access" /discover/listings | jq -e 'length >= 1' >/dev/null

current_step="complete"
echo "PASS: email login -> Explore -> enquiry -> invitation -> manual KYC -> atomic activation -> tenant context -> Explore"
