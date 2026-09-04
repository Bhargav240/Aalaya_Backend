#!/bin/bash
# End-to-end Rev-3 flow test against a fresh hamsa_e2e DB (cooldown=0, vacancy 5s).
B=http://127.0.0.1:8091/api/v1
PASS=0; FAIL=0
step() { echo "=== $* ==="; }
ok()   { PASS=$((PASS+1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL+1)); echo "  FAIL: $1"; }
check() { if [ "$2" = "$3" ]; then ok "$1"; else fail "$1 -- expected [$2] got [$3]"; fi; }
last_code() { sed 's/\x1b\[[0-9;]*m//g' /tmp/pgsrv.log | grep -a "otp sent" | tail -1 | sed -E 's/.*code[^0-9]*([0-9]{6}).*/\1/'; }
otp_login() { # $1 phone  $2 device  -> echoes access_token
  local code resp
  curl -sS -X POST $B/auth/otp/request -H 'content-type: application/json' -d "{\"phone\":\"$1\"}" >/dev/null
  sleep 0.25
  code=$(last_code)
  curl -sS -X POST $B/auth/otp/verify -H 'content-type: application/json' -d "{\"phone\":\"$1\",\"code\":\"$code\",\"device_id\":\"$2\"}"
}

step "1. provision org (phone-keyed)"
R=$(curl -sS -X POST $B/orgs -H 'x-provisioning-key: e2e-prov' -H 'content-type: application/json' \
  -d '{"name":"Hamsa Vilas","owner_phone":"9876500001","owner_name":"Priya Owner"}')
ORG=$(echo "$R" | python3 -c "import sys,json;print(json.load(sys.stdin)['org_id'])" 2>/dev/null)
[ -n "$ORG" ] && ok "provision org $ORG" || fail "provision: $R"

step "2. owner OTP login, first-ever device"
R=$(otp_login 9876500001 dev-owner-1)
AT=$(echo "$R" | python3 -c "import sys,json;print(json.load(sys.stdin)['access_token'])" 2>/dev/null)
ND=$(echo "$R" | python3 -c "import sys,json;print(json.load(sys.stdin)['is_new_device'])" 2>/dev/null)
[ -n "$AT" ] && ok "owner login" || fail "owner login: $R"
check "first login is_new_device" "True" "$ND"

step "3. repeat login, same trusted device"
R=$(otp_login 9876500001 dev-owner-1)
ND=$(echo "$R" | python3 -c "import sys,json;print(json.load(sys.stdin)['is_new_device'])" 2>/dev/null)
check "same device is_new_device false" "False" "$ND"

step "4. branches"
BRS=$(curl -sS $B/branches -H "Authorization: Bearer $AT")
BR=$(echo "$BRS" | python3 -c "import sys,json;print(json.load(sys.stdin)[0]['id'])" 2>/dev/null)
[ -n "$BR" ] && ok "branch $BR" || fail "branches: $BRS"

step "5. room + 2 beds"
R=$(curl -sS -X POST $B/branches/$BR/rooms -H "Authorization: Bearer $AT" -H 'content-type: application/json' -d '{"number":"101"}')
ROOM=$(echo "$R" | python3 -c "import sys,json;print(json.load(sys.stdin)['id'])" 2>/dev/null)
for L in A B; do curl -sS -X POST $B/rooms/$ROOM/beds -H "Authorization: Bearer $AT" -H 'content-type: application/json' -d "{\"label\":\"$L\"}" >/dev/null; done
N=$(curl -sS $B/rooms/$ROOM/beds -H "Authorization: Bearer $AT" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")
check "2 beds" "2" "$N"
BEDS=$(curl -sS $B/rooms/$ROOM/beds -H "Authorization: Bearer $AT")

step "6. resident auto-provision + bed assign + ledger deposit"
R=$(curl -sS -X POST $B/branches/$BR/residents -H "Authorization: Bearer $AT" -H 'content-type: application/json' \
  -d '{"full_name":"Raj Tenant","phone":"9876500002","plan_kind":"monthly","rent_paise":500000,"deposit_paise":1000000}')
RES=$(echo "$R" | python3 -c "import sys,json;print(json.load(sys.stdin)['id'])" 2>/dev/null)
[ -n "$RES" ] && ok "resident $RES" || fail "resident: $R"
BED=$(echo "$BEDS" | python3 -c "import sys,json;print(json.load(sys.stdin)[0]['id'])")
curl -sS -X POST $B/residents/$RES/assign-bed -H "Authorization: Bearer $AT" -H 'content-type: application/json' -d "{\"bed_id\":\"$BED\"}" >/dev/null
L=$(curl -sS $B/residents/$RES/ledger -H "Authorization: Bearer $AT")
echo "$L" | grep -q '"deposit_paise":1000000' && ok "ledger deposit 1000000" || fail "ledger: $L"

step "7. resident phone signs in (no invite)"
R=$(otp_login 9876500002 dev-tenant-1)
TAT=$(echo "$R" | python3 -c "import sys,json;print(json.load(sys.stdin)['access_token'])" 2>/dev/null)
[ -n "$TAT" ] && ok "resident phone login" || fail "resident login: $R"
L=$(curl -sS $B/me/ledger -H "Authorization: Bearer $TAT")
echo "$L" | grep -q '"deposit_paise":1000000' && ok "tenant sees own deposit" || fail "tenant ledger: $L"

step "8. record payment + receipt"
curl -sS -X POST $B/residents/$RES/payments -H "Authorization: Bearer $AT" -H 'content-type: application/json' -d '{"amount_paise":500000}' >/dev/null
L=$(curl -sS $B/residents/$RES/ledger -H "Authorization: Bearer $AT")
echo "$L" | grep -q '"paid_total_paise":500000' && ok "payment recorded" || fail "payment: $L"

step "9. listing lifecycle + discovery gate"
curl -sS -X PUT $B/listings/$BR -H "Authorization: Bearer $AT" -H 'content-type: application/json' \
  -d '{"title":"Hamsa Vilas Kora","area":"Koramangala","city":"Bengaluru","lat":12.93,"lng":77.62,"description":"Nice PG","amenities":"wifi,meals"}' >/dev/null
ST=$(curl -sS $B/listings -H "Authorization: Bearer $AT" | python3 -c "import sys,json;print(json.load(sys.stdin)[0]['status'])")
check "draft after prepare" "draft" "$ST"
curl -sS -X POST $B/listings/$BR/submit -H "Authorization: Bearer $AT" >/dev/null
ST=$(curl -sS $B/listings -H "Authorization: Bearer $AT" | python3 -c "import sys,json;print(json.load(sys.stdin)[0]['status'])")
check "pending after submit" "pending" "$ST"
N=$(curl -sS "$B/discover/listings" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")
check "public discovery hidden pre-publish" "0" "$N"

step "10. platform operator gating + publish"
psql -d hamsa_e2e -q \
  -c "INSERT INTO public.organizations(id,name) VALUES ('00000000-0000-0000-0000-000000000001','Platform') ON CONFLICT DO NOTHING;" \
  -c "INSERT INTO public.platform_users (id,org_id,phone,full_name,status) VALUES ('11111111-1111-4111-8111-111111111111','00000000-0000-0000-0000-000000000001','9999999999','Ops','active');" \
  -c "INSERT INTO public.user_org_roles (user_id,org_id,role) VALUES ('11111111-1111-4111-8111-111111111111','00000000-0000-0000-0000-000000000001','platform_operator');"
OAT=$(otp_login 9999999999 dev-ops-1 | python3 -c "import sys,json;print(json.load(sys.stdin)['access_token'])" 2>/dev/null)
[ -n "$OAT" ] && ok "operator login" || fail "operator login"
HTTP=$(curl -sS -o /dev/null -w "%{http_code}" -X POST $B/operator/listings/$BR/publish -H "Authorization: Bearer $AT")
check "owner cannot publish (403)" "403" "$HTTP"
Q=$(curl -sS $B/operator/listings -H "Authorization: Bearer $OAT" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")
check "operator queue has 1" "1" "$Q"
curl -sS -X POST $B/operator/listings/$BR/publish -H "Authorization: Bearer $OAT" >/dev/null
ST=$(curl -sS $B/listings -H "Authorization: Bearer $AT" | python3 -c "import sys,json;print(json.load(sys.stdin)[0]['status'])")
check "live after operator publish" "live" "$ST"

step "11. anonymous discovery + vacancy + enquire"
sleep 7  # vacancy sweep (5s)
D=$(curl -sS "$B/discover/listings")
echo "  $D"
V=$(echo "$D" | python3 -c "import sys,json;d=json.load(sys.stdin);print(d[0].get('beds_available'),d[0].get('starting_price_paise'))" 2>/dev/null)
check "vacancy 1 free @ Rs5000" "1 500000" "$V"
S=$(curl -sS "$B/discover/search?q=Kora" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")
check "search finds listing" "1" "$S"
N=$(curl -sS "$B/discover/nearby?lat=12.9&lng=77.6" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")
check "nearby finds listing" "1" "$N"
# anonymous enquiry -> prospect + lead
curl -sS -X POST $B/enquiries/otp/request -H 'content-type: application/json' -d '{"phone":"9888888888"}' >/dev/null
sleep 0.25
CODE=$(last_code)
R=$(curl -sS -X POST $B/listings/$BR/enquire -H 'content-type: application/json' -d "{\"phone\":\"9888888888\",\"code\":\"$CODE\",\"note\":\"need 1 bed\"}")
echo "$R" | grep -q received && ok "anonymous enquiry lead + prospect" || fail "enquire: $R"

step "12. manager inbox + owner org-wide view"
psql -d hamsa_e2e -q \
  -c "INSERT INTO public.platform_users (id,org_id,phone,full_name,status) VALUES (gen_random_uuid(),'$ORG','9876500003','Manoj Manager','active');" \
  -c "INSERT INTO public.user_org_roles (user_id,org_id,role,branch_id) SELECT id,org_id,'manager','$BR' FROM public.platform_users WHERE phone='9876500003';"
MAT=$(otp_login 9876500003 dev-mgr-1 | python3 -c "import sys,json;print(json.load(sys.stdin)['access_token'])" 2>/dev/null)
[ -n "$MAT" ] && ok "manager login" || fail "manager login"
ENC=$(curl -sS $B/enquiries -H "Authorization: Bearer $MAT" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d),d[0]['phone'] if d else '')")
check "manager inbox 1 lead" "1 9888888888" "$ENC"
ENC2=$(curl -sS $B/enquiries -H "Authorization: Bearer $AT" | python3 -c "import sys,json;print(len(json.load(sys.stdin)))")
check "owner org-wide sees lead" "1" "$ENC2"

echo; echo "RESULT: $PASS passed, $FAIL failed"
