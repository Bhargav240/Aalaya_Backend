#!/usr/bin/env bash
# Disposable live test: 1,000 residents + GET latency + owner/manager/tenant scope.
# Requires: pg-server on http://127.0.0.1:8080, psql, node, and .env containing
# MIGRATION_DATABASE_URL + JWT_SECRET. It never prints either secret.
set -euo pipefail

api_base="${API_BASE_URL:-http://127.0.0.1:8080/api/v1}"
db_url="$(sed -n 's/^MIGRATION_DATABASE_URL=//p' .env | head -n 1)"
jwt_secret="$(sed -n 's/^JWT_SECRET=//p' .env | head -n 1)"
[[ -n "$db_url" && -n "$jwt_secret" ]] || { echo 'MIGRATION_DATABASE_URL and JWT_SECRET are required in .env'; exit 2; }

org=31000000-0000-0000-0000-000000000001
branch_a=31000000-0000-0000-0000-000000000002
branch_b=31000000-0000-0000-0000-000000000003
room_a=31000000-0000-0000-0000-000000000004
bed_a=31000000-0000-0000-0000-000000000005
tenant_resident=31000000-0000-0000-0000-000000000006
owner=31000000-0000-0000-0000-000000000011
manager=31000000-0000-0000-0000-000000000012
tenant=31000000-0000-0000-0000-000000000013
missing_receipt=31000000-0000-0000-0000-000000000099
report="${TMPDIR:-/tmp}/pg-scale-get-api-$(date +%Y%m%d-%H%M%S).csv"
body="${TMPDIR:-/tmp}/pg-scale-get-api-body.json"
pass=0; fail=0

cleanup() {
  psql "$db_url" -v ON_ERROR_STOP=1 >/dev/null <<SQL || true
BEGIN;
SET LOCAL search_path TO pool, public, extensions;
SELECT set_config('app.current_org', '$org', true);
DELETE FROM move_out_deductions WHERE settlement_id IN (SELECT id FROM move_out_settlements WHERE org_id = '$org');
DELETE FROM move_out_settlements WHERE org_id = '$org';
DELETE FROM away_periods WHERE org_id = '$org';
DELETE FROM receipts WHERE org_id = '$org';
DELETE FROM payments WHERE org_id = '$org';
DELETE FROM complaint_events WHERE org_id = '$org';
DELETE FROM complaints WHERE org_id = '$org';
DELETE FROM notices WHERE org_id = '$org';
DELETE FROM change_requests WHERE org_id = '$org';
DELETE FROM audit_log WHERE org_id = '$org';
DELETE FROM archive_manifest WHERE org_id = '$org';
DELETE FROM residents WHERE org_id = '$org';
DELETE FROM beds WHERE org_id = '$org';
DELETE FROM rooms WHERE org_id = '$org';
DELETE FROM branches WHERE org_id = '$org';
DELETE FROM public.user_org_roles WHERE user_id IN ('$owner', '$manager', '$tenant');
DELETE FROM public.platform_users WHERE id IN ('$owner', '$manager', '$tenant');
DELETE FROM public.tenant_placement WHERE org_id = '$org';
DELETE FROM public.organizations WHERE id = '$org';
COMMIT;
SQL
}
trap cleanup EXIT

curl -fsS --max-time 10 http://127.0.0.1:8080/health >/dev/null || { echo 'pg-server is not healthy on port 8080'; exit 2; }
cleanup

psql "$db_url" -v ON_ERROR_STOP=1 >/dev/null <<SQL
BEGIN;
INSERT INTO public.organizations (id, name) VALUES ('$org', 'Scale QA — delete after test');
INSERT INTO public.tenant_placement (org_id, mode, schema_name) VALUES ('$org', 'pooled', 'pool');
INSERT INTO public.platform_users (id, org_id, phone, full_name, status) VALUES
 ('$owner', '$org', '7010000001', 'Scale Owner', 'active'),
 ('$manager', '$org', '7010000002', 'Scale Manager', 'active'),
 ('$tenant', '$org', '7010000003', 'Scale Tenant', 'active');
INSERT INTO public.user_org_roles (user_id, org_id, role, branch_id) VALUES
 ('$owner', '$org', 'owner', NULL),
 ('$manager', '$org', 'manager', '$branch_a'),
 ('$tenant', '$org', 'tenant', '$branch_a');
SET LOCAL search_path TO pool, public, extensions;
SELECT set_config('app.current_org', '$org', true);
INSERT INTO branches (id, org_id, name) VALUES ('$branch_a', '$org', 'Scale Main'), ('$branch_b', '$org', 'Scale Secondary');
INSERT INTO rooms (id, org_id, branch_id, number, cooling_type) VALUES ('$room_a', '$org', '$branch_a', '1001', 'ac');
INSERT INTO beds (id, org_id, room_id, label) VALUES ('$bed_a', '$org', '$room_a', 'A');
INSERT INTO residents (id, org_id, branch_id, bed_id, user_id, full_name, phone, status, plan_kind, rent_paise) VALUES
 ('$tenant_resident', '$org', '$branch_a', '$bed_a', '$tenant', 'Scale Tenant', '7010000003', 'active', 'monthly', 1000000);
INSERT INTO residents (id, org_id, branch_id, full_name, phone, status, plan_kind, rent_paise)
SELECT gen_random_uuid(), '$org', '$branch_a', 'Scale Resident ' || g,
       '80' || lpad(g::text, 8, '0'), 'active', 'monthly', 1000000
FROM generate_series(1, 999) AS g;
COMMIT;
SQL

token() {
  JWT_SECRET="$jwt_secret" TEST_UID="$1" TEST_ORG="$org" TEST_ROLE="$2" node -e '
    const c=require("crypto"), b=x=>Buffer.from(x).toString("base64url");
    const now=Math.floor(Date.now()/1000), h=b(JSON.stringify({alg:"HS256",typ:"JWT"}));
    const p=b(JSON.stringify({sub:process.env.TEST_UID,org:process.env.TEST_ORG,role:process.env.TEST_ROLE,kind:"access",iat:now,exp:now+900,jti:c.randomUUID()}));
    process.stdout.write(`${h}.${p}.${c.createHmac("sha256",process.env.JWT_SECRET).update(`${h}.${p}`).digest("base64url")}`);'
}
owner_token="$(token "$owner" owner)"; manager_token="$(token "$manager" manager)"; tenant_token="$(token "$tenant" tenant)"
printf 'case,expected,status,latency_seconds,bytes\n' > "$report"

check_get() {
  local label="$1" expected="$2" bearer="$3" path="$4"
  local metrics status latency bytes
  if [[ -n "$bearer" ]]; then
    metrics="$(curl -sS --max-time 30 -o "$body" -w '%{http_code},%{time_total},%{size_download}' -H "Authorization: Bearer $bearer" "$api_base$path")"
  else
    metrics="$(curl -sS --max-time 30 -o "$body" -w '%{http_code},%{time_total},%{size_download}' "$api_base$path")"
  fi
  IFS=, read -r status latency bytes <<< "$metrics"
  printf '%s,%s,%s,%s,%s\n' "$label" "$expected" "$status" "$latency" "$bytes" >> "$report"
  if [[ "$status" == "$expected" ]]; then ((pass+=1)); else echo "FAIL $label: expected $expected, got $status"; ((fail+=1)); fi
}

# Public GET surface. (Root-level /health is checked before fixtures are made.)
check_get discover_list 200 '' '/discover/listings?limit=200'
check_get discover_search 200 '' '/discover/search?q=Scale&limit=200'
check_get discover_nearby 200 '' '/discover/nearby?lat=12.9716&lng=77.5946&limit=50'
check_get discover_hidden_branch 404 '' "/discover/listings/$branch_a"

# Owner sees all management data. The residents response is the 1,000-record latency measurement.
check_get owner_me 200 "$owner_token" '/me'
check_get owner_branches 200 "$owner_token" '/branches'
check_get owner_rooms 200 "$owner_token" "/branches/$branch_a/rooms"
check_get owner_beds 200 "$owner_token" "/branches/$branch_a/beds"
check_get owner_room_beds 200 "$owner_token" "/rooms/$room_a/beds"
check_get owner_residents_1000 200 "$owner_token" "/branches/$branch_a/residents"
node -e 'const fs=require("fs"); const n=JSON.parse(fs.readFileSync(process.argv[1])).length; if(n!==1000){process.exitCode=1; console.error(`FAIL expected 1000 residents, got ${n}`)}' "$body" || ((fail+=1))
check_get owner_dues 200 "$owner_token" "/branches/$branch_a/dues"
check_get owner_resident_ledger 200 "$owner_token" "/residents/$tenant_resident/ledger"
check_get owner_resident_receipts 200 "$owner_token" "/residents/$tenant_resident/receipts"
check_get owner_changes 200 "$owner_token" '/change-requests'
check_get owner_complaints 200 "$owner_token" '/complaints'
check_get owner_notices 200 "$owner_token" '/notices'
check_get owner_away 200 "$owner_token" '/away-periods'
check_get owner_finance_branch 200 "$owner_token" "/finance/branch/$branch_a"
check_get owner_finance_org 200 "$owner_token" '/finance/org'
check_get owner_template 200 "$owner_token" '/org/receipt-template'
check_get owner_audit 200 "$owner_token" '/audit?limit=500'
check_get owner_enquiries 200 "$owner_token" '/enquiries'
check_get owner_listings 200 "$owner_token" '/listings'
check_get owner_missing_receipt_download 404 "$owner_token" "/receipts/$missing_receipt/download"

# Manager receives only their branch and cannot cross into the other branch.
check_get manager_me 200 "$manager_token" '/me'
check_get manager_branches 200 "$manager_token" '/branches'
check_get manager_own_residents 200 "$manager_token" "/branches/$branch_a/residents"
check_get manager_other_residents 403 "$manager_token" "/branches/$branch_b/residents"
check_get manager_own_finance 200 "$manager_token" "/finance/branch/$branch_a"
check_get manager_other_finance 403 "$manager_token" "/finance/branch/$branch_b"
check_get manager_org_finance 403 "$manager_token" '/finance/org'
check_get manager_enquiries 200 "$manager_token" '/enquiries'

# Tenant gets self-service resources only, never operational organization data.
check_get tenant_me 200 "$tenant_token" '/me'
check_get tenant_ledger 200 "$tenant_token" '/me/ledger'
check_get tenant_receipts 200 "$tenant_token" '/me/receipts'
check_get tenant_away 200 "$tenant_token" '/me/away-periods'
check_get tenant_missing_receipt_download 404 "$tenant_token" "/me/receipts/$missing_receipt/download"
check_get tenant_branches_forbidden 403 "$tenant_token" '/branches'
check_get tenant_rooms_forbidden 403 "$tenant_token" "/branches/$branch_a/rooms"
check_get tenant_residents_forbidden 403 "$tenant_token" "/branches/$branch_a/residents"
check_get tenant_finance_forbidden 403 "$tenant_token" '/finance/org'
check_get tenant_listings_forbidden 403 "$tenant_token" '/listings'

echo "Scale GET/API role test: $pass passed, $fail failed"
echo "Latency report: $report"
[[ "$fail" == 0 ]]
