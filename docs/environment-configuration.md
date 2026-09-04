# Environment configuration

Copy `.env.example` to `.env`. Never commit `.env`, database passwords, JWT
secrets, Resend keys, DigiLocker secrets, Supabase service-role keys, or OTPs.

## Required in every deployed environment

| Variable | Purpose |
|---|---|
| `PROFILE` | Use `development` locally and `production` in production. Production enables fail-fast safety checks. |
| `DATABASE_URL` | Runtime SQLx URL. For Supabase use the transaction pooler. Percent-encode special password characters. |
| `MIGRATION_DATABASE_URL` | Direct/session-pooler URL used only for migrations and integration tests. |
| `JWT_SECRET` | Long random HS256 signing secret. Use a different value per environment. |
| `PROVISIONING_KEY` | Protects the back-office `POST /orgs` endpoint. |
| `PLATFORM_ORG_ID` | Control-plane organization used by authenticated Explorer/prospect accounts. |
| `BIND_ADDR` | Listener, normally `0.0.0.0:8080`. |

Generate secrets with a password manager or `openssl rand -base64 48`. Rotate a
secret if it was pasted into chat, source control, build logs, or an issue.

## Email OTP and invitation email

| Variable | Recommended value |
|---|---|
| `EMAIL_PROVIDER` | `log` for local development; `resend` for production. |
| `RESEND_API_KEY` | Resend API key scoped to email sending. Required in production. |
| `EMAIL_FROM` | Verified sender, for example `Hamsa Vilas <no-reply@example.com>`. |
| `DEBUG_EMAIL_OTP` | `true` only for local debug builds. Release builds ignore it. |
| `OTP_TTL_SECS` | `600` (10 minutes). |
| `OTP_RESEND_COOLDOWN_SECS` | `30`. |
| `OTP_MAX_ATTEMPTS` | `5`. |
| `EMAIL_OTP_MAX_SENDS` | `3`. |
| `EMAIL_OTP_WINDOW_SECS` | `900` (15 minutes). |
| `APP_DEEP_LINK_BASE` | `hamsavilas://onboarding`. |
| `INVITATION_RESEND_COOLDOWN_SECS` | `60` or higher. |

Production startup intentionally fails when `EMAIL_PROVIDER` is not `resend`
or when `RESEND_API_KEY`/`EMAIL_FROM` is missing. Login OTPs are Argon2-hashed;
the raw code must never be logged outside the double-gated local debug mode.

## DigiLocker

Set all of these together in production:

- `DIGILOCKER_CLIENT_ID`
- `DIGILOCKER_CLIENT_SECRET`
- `DIGILOCKER_REDIRECT_URI` — HTTPS backend callback registered with DigiLocker
- `DIGILOCKER_SESSION_ENCRYPTION_KEY` — long random secret
- `DIGILOCKER_AUTHORIZATION_ENDPOINT`
- `DIGILOCKER_TOKEN_ENDPOINT`
- `DIGILOCKER_USERINFO_ENDPOINT`
- `DIGILOCKER_SCOPES`
- `DIGILOCKER_SESSION_TTL_SECS=600`

The Android app never receives the client secret or provider access token.

## Organization layout

Set `ONBOARDING_LAYOUT_PATH=./config/onboarding-layout.example.yaml` to create
the client-specific floors, rooms, bed counts, AC/non-AC type, and identity
policy when an organization/branch is onboarded. Reconciliation is additive:
rooms and beds added later in Android remain available and are not deleted.

## Storage and optional integrations

- `STORAGE_BACKEND=local` and `STORAGE_DIR=./data/objectstore` for development.
- `STORAGE_BACKEND=supabase` plus `SUPABASE_URL`, `SUPABASE_SERVICE_KEY`,
  `SUPABASE_BUCKET` for deployed receipt storage. The service-role key is
  backend-only.
- `FCM_SERVICE_ACCOUNT_JSON` enables push notices. Do not put it in Android.
- `GOOGLE_CLIENT_ID` is optional legacy Google sign-in; email OTP remains the
  primary login path.

For deployment use the platform's secret manager. Keep separate development,
staging, and production Supabase projects and credentials.
