# Test coverage

Status of the backend test harness. Run `make test` to exercise everything.

## Suites

| Test target | Tests | What it verifies | Why it matters |
|---|---:|---|---|
| `crates/application/tests/money_tests.rs` | 13 | Move-out settlement math (`short notice → full forfeit`; `full notice → deposit − outstanding − deductions, floored at 0`; `second move-out → 409`; accrual stops at `moved_out_at`; outstanding caps at 0; receipt numbering `RC-XXXXXXXX`; receipt branch-line shown only for multi-branch orgs; non-manager RBAC; zero/negative amounts rejected). | Rent math IS the product. Numbers asserted here come from the real production `SimpleBillingEngine` via the `pg-infrastructure` dev-dep. |
| `crates/application/tests/otp_tests.rs` | 9 | Silent Ok for unknown/inactive phones (no user-enumeration leak); resend-cooldown `Validation`; happy path; correct-code verify → token pair; wrong code → `Unauthorized` + attempt counter; exhausted attempts → locked out even for the right code; missing challenge → `Unauthorized`; cross-user challenge steal → `Unauthorized`. | Indistinguishable errors are the security posture — if a wrong code ever returned a different error class than "no challenge," an attacker could enumerate phones. |
| `crates/application/tests/auth_session_tests.rs` | 8 | `normalize_phone` (strips to last 10 digits; <10 digits → `None`); refresh rotation (live token rotated + old hash killed; replaying the OLD refresh token is 401; `access` kind rejected; unknown tokens 401); org-provision happy path. | A stolen refresh token must die on first legitimate use; this is the whole point of rotating refresh tokens. |
| `crates/application/tests/maker_checker_tests.rs` | 5 | Owner edits apply immediately (`Applied`); manager edits write a `change_requests` row and NEVER mutate (`Pending`); tenant edits are forbidden; both `update` and `delete` route identically. | The core governance rule of the whole product. A single regression here is a manager silently getting owner power. |
| `crates/application/tests/prospect_tests.rs` | 4 | Anonymous enquiry OTP for unknown phone creates a `null-user` challenge; `verify_enquiry` auto-creates a Prospect + lead; an already-registered phone reuses the user (no duplicate Prospect); wrong code → `Unauthorized` and NO lead is written. | The marketplace funnel is the top of the tenant pipeline — a bug here either loses leads or lets a stranger proxy-enquire on someone else's phone. |

Total: **47** unit tests (`cargo test`-native, no testcontainers, <1s) **+ a 25-assertion end-to-end script** (`scripts/e2e.sh`) that boots the real server against a live Postgres and walks every Rev-3 flow (provision → OTP → trusted device → resident auto-provision → ledger → payment → listing draft→pending→operator_publish→live → vacancy projected → anonymous enquiry → manager/owner inbox).

## Status per application service

| Service file | Test file | Status | Notes |
|---|---|---|---|
| `services/mod.rs` (auth, OTP, provision, invites, refresh, normalize_phone) | `auth_session_tests.rs`, `otp_tests.rs` | ✅ Mostly covered | `create_invite` + `redeem_invite` exercised via happy path; edge branches on expired invites untested. |
| `services/money.rs` (record_payment, ledger, move_out, my_receipts, receipt_resident, templates) | `money_tests.rs` | ✅ Core paths | Receipt template GET/PUT passes through; signed-URL branching untested. |
| `services/core.rs` (CRUD, edit_entity, maker-checker, link_resident_user) | `maker_checker_tests.rs` | ✅ Maker-checker fully | `assign_bed` cross-branch guard + occupied-bed logic untested. CRUD helpers pass through. |
| `services/complaints.rs` | — | ⚠️ Indirectly covered | State machine lives in domain, Android has parallel unit tests. Backend transition RBAC + bounce-back untested. |
| `services/verification.rs` | — | ❌ Untested | Name-match gate, record-attempt audit trail untested. |
| `services/archive.rs` | — | ❌ Untested | Owner-only archive; needs ObjectStore spy + deterministic clock. |
| `services/provisioning.rs` | — | ⚠️ Indirectly | `promote_to_silo` owner-only guard untested. |
| `services/discovery.rs` | — | ✅ E2E | Unit: pass-through. Lifecycle (draft→pending→operator-publish→live), 403 for non-operator publish, discovery gating — covered by `scripts/e2e.sh`. |
| `services/enquiries.rs` | `prospect_tests.rs` | ✅ Unit + E2E | Anonymous OTP, auto-Prospect, lead write, manager-branch vs owner-org inbox scoping (E2E `scripts/e2e.sh`). |
| `services/vacancy.rs` | — | ✅ E2E | Background sweep derives beds/price from tenant schema into `listing_vacancy`; discovery JOIN verified live in E2E. |
| `services/finance.rs` | — | ⚠️ Indirectly | Branch-finance RBAC untested. |

## Recently shipped features

| Feature | Where | Notes |
|---|---|---|
| **Revision-3 sign-in model** | `platform_users.phone` is the unique identity; `password_hash`, `invites` table, and invite/redeem endpoints removed | Sign-in is phone+OTP only; registration (by manager/owner, or marketplace enquiry) IS the invitation. `create_resident` auto-provisions/links the platform user; a marketplace Prospect is promoted to Tenant on the same phone. |
| **Trusted device flag** | `trusted_devices` table; `POST /auth/otp/verify` accepts `device_id`/`device_name` and responds with `{is_verified, is_new_device}` | A sign-in from unknown hardware flips `is_new_device` so the app can ask for extra confirmation. |
| **Platform Operator listing gate** | `public_listings.status` in `draft → pending → live / rejected / unlisted`; operator-only `POST /operator/listings`, `.../publish`, `.../reject`, `.../unlist`; owner/manager use `PUT /listings/{branch}` + `POST /listings/{branch}/submit` | Owners/managers PREPARE; only the platform operator publishes. |
| **Live vacancy** | `public.listing_vacancy` snapshot (beds_total, beds_available, starting_price_paise, stale) refreshed by a background sweep (`VACANCY_INTERVAL_SECS`) from real tenant bed data | Discovery queries JOIN the snapshot — vacancy can never go stale from someone forgetting to update it. |
| **Enquiry funnel** | `POST /enquiries/otp/request` + `POST /listings/{branch}/enquire` (anonymous); `GET /enquiries` (manager: own branch; owner: whole org); `POST /enquiries/{id}/status` | Prospect = light platform user in `PLATFORM_ORG_ID`; OTP proves the phone before a lead exists. |
| **Rent & deposit editable** | `PATCH /residents/{id}` accepts `rent_paise`/`deposit_paise`, routed through maker-checker like every other edit | Manager edits queue for owner approval; annual rent revisions are routine. |
| **Delete a bed** | `bed` added to the maker-checker entity map (soft-delete) | Same approval flow as rooms/residents. |
| **Push notifications (FCM)** | `POST/DELETE /me/devices` register FCM tokens in `device_tokens`; `FcmNotifier` signs OAuth2 JWTs from `FCM_SERVICE_ACCOUNT_JSON` | Log fallback in dev. |
| **SMS providers** | `SMS_PROVIDER=log|msg91|twilio` selects OTP delivery (`MSG91_*` / `TWILIO_*` env) | Log fallback when creds absent so dev never blocks on SMS. |
| **Archive scheduler** | Server binary runs an interval task (`ARCHIVE_INTERVAL_SECS`, `ARCHIVE_OLDER_THAN_DAYS`) executing `archive_deleted` for every org | Manual `POST /admin/archive` still available. |
| **Away days** | `away_periods` table (tenant), `POST/GET /away-periods`, `GET/DELETE /me/away-periods` | Purely a **safety notification** — never changes rent (boundary-tested). |
| **Security/reliability hardening** | `PROFILE=production` fail-fast (JWT secret, `VERIFICATION_METHOD≠mock`, `DATABASE_URL`); refresh rotation re-reads role from DB (no stale-role minting); expired-OTP `consume` is atomic; `Idempotency-Key` on `POST /payments`; per-bed partial unique index; tenant-router 30s TTL + invalidate on promotion | See the audit pass in git history — closes the P0/P1 findings. |
| **Batch dues / payments perf** | `GET /branches/{id}/dues` rolls every live resident into one SQL query (accrual formula shared with `SimpleBillingEngine`); payments screen no longer fans out N `ledger` calls | Android `PaymentsViewModel.refresh` switched to fetch it in parallel with the resident roster. |
| **Append-only sweeps** | `otp_codes` and `refresh_tokens` get a daily cleanup (24h interval on the worker pool) — `DELETE expired OR revoked` so they never grow unbounded | Plus 5s/3s reqwest timeouts on SMS/FCM clients; vacancy sweep jittered + start-offset randomized. |
| **Bounded reads** | `complaints`/`enquiries`/`notices`/`listings`/`change_requests` list queries now carry `LIMIT 500` — keeps response size sane while the UI has no pagination yet | Trigram GIN + geo bbox + per-expiry indexes in control-plane; `residents_accrual` + `beds_org_room` covering indexes in tenant. |
| **Batch placements** | `GET /branches/{id}/beds` joins rooms→beds in one query and returns the occupant — one call replaces the payments screen's per-room fan-out | Live-verified: returns `Raj Tenant`'s occupancy + an empty neighbouring bed. |
| **Access-token revocation** | `Claims` now carries `jti` + `exp`; `POST /auth/logout` blocklists the jti in-process and the `AuthUser` extractor rejects a revoked token even before its `exp` — the "stolen access token works until TTL" window is closed | Live-verified: a token that was 200s before logout is 401 immediately after. Opportunistic GC keeps the map bounded to live-ttl size. |
| **Migration bookkeeping** | `schema_migrations_tenant` now records both `pool/0001_init` and `public/0001_init` so future versioned migrations can key off schema_migrations instead of re-running IF NOT EXISTS DDL every boot | Matches the existing advisory-locked migration runner. |
| **Google sign-in** | `POST /auth/google` verifies RS256 ID tokens against Google's JWKS | Only a previously-bound `google_sub` signs in; Google never auto-provisions anyone. |
| **Move-out settlement persistence** | `move_out_settlements` + `move_out_deductions`; persisted atomically with the status flip | Settlements are immutable history. |

## Hard boundaries of the current harness

- Unit suites have no Postgres / testcontainers participation: all DB access there happens behind fakes under `crates/application/tests/fakes/`.
- **End-to-end Rev-3 flows ARE exercised against a live Postgres** by the bash script `scripts/e2e.sh` (25 assertions: provision → OTP → trusted-device → resident auto-provision → ledger → payment → listing lifecycle → operator gating → vacancy projection → anonymous enquiry → manager/owner inbox). It is a *script*, not yet wired into `cargo test` — Phase 2 is porting it to `axum-test` + testcontainers so CI can run it.
- Axum extractor plumbing (`AuthUser`, `resolve_ctx`) is exercised by `scripts/e2e.sh` over HTTP, but not yet by a `cargo test` integration suite.
- No OpenAPI contract tests (because there is no OpenAPI spec yet).
- UI / Android: separate test suite (Jest-based under `android_kotlin/app/__tests__/`).

## Coding rules

- One suite per `services/<file>.rs`. Match names: `services/money.rs` → `tests/money_tests.rs`.
- Fakes in `tests/fakes/`, NEVER inline in a test file. Extend existing fakes by adding methods; never per-test branching.
- Reuse helpers in `fakes/` (`FixedClock`, `SpyOtpSender`, `SpyRenderer`, `FakeTokenService`, `Fake*Repo`) before reaching for new ones.
- Test names: `<unit>__<scenario>__<expected>` (double underscore). `#![allow(non_snake_case)]` at file top.
- Time-dependent logic: inject `FixedClock`; never `OffsetDateTime::now_utc()` inside a test assertion.
- Numbers asserted: prefer real production values via `SimpleBillingEngine`/`sha256_hex`, never re-implemented formulas.
- Single-flow guarantees are what you test, not the whole world — an `assert_eq!` count of spies (sender.send called once, changed once, etc.) goes further than twenty byte-level assertions on incidental data.

## What would be Phase 2 (intentionally not done in this wave)

- Real-Postgres integration tests via `testcontainers` exercising the SQLx adapters, `scoped_tx`, RLS, pool→silo promotion round-trip. This is the main coverage hole for the tenancy/RLS story.
- HTTP contract tests via `axum-test` and a seeded ephemeral Postgres. Validates the `AuthUser` extractor + RBAC + branch-scope guards hit the right branch on every route.
- OpenAPI spec generation + contract tests against it.
- GitHub Actions workflow that runs `make test` on every PR (currently nothing in `.github/`).
