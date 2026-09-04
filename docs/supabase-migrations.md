# Supabase migration workflow

The Rust backend is the only application client of PostgreSQL. Android calls
the Rust API; it does not receive a service-role key or query operational
Supabase tables directly.

## One-time setup

```bash
cd /path/to/backend
npx supabase login
npx supabase link --project-ref <project-reference-id>
npx supabase start
```

Use Singapore when that is the project's chosen region. Put the runtime
transaction-pooler URL in `DATABASE_URL` and the direct/session-pooler URL in
`MIGRATION_DATABASE_URL`. Percent-encode special password characters.

## Validate locally before remote push

```bash
npx supabase db reset --local
```

Expected migrations, in order:

1. control-plane baseline
2. pooled tenant schema and RLS
3. legacy baseline marker
4. email authentication
5. tenant invitations
6. membership status
7. invitation constraints
8. invitation delivery metadata
9. membership/resident links
10. DigiLocker and manual KYC

Then run isolation tests with the privileged local maintenance URL:

```bash
export MIGRATION_DATABASE_URL='postgresql://postgres:postgres@127.0.0.1:54322/postgres'
cargo test -p pg-infrastructure --test tenant_activation_tests
psql "$MIGRATION_DATABASE_URL" -v ON_ERROR_STOP=1 -f supabase/tests/tenant_rls.sql
psql "$MIGRATION_DATABASE_URL" -v ON_ERROR_STOP=1 -f supabase/tests/multitenancy_deep.sql
```

All SQL tests use transactions and roll back their dummy rows.

## Push safely

```bash
npx supabase db push --dry-run
npx supabase db push
```

Read the dry-run list before approving the real push. Do not edit a migration
already applied to a shared environment; add a new forward migration instead.
Never use `db reset` against the remote production project.

The legacy `public.otp_codes` table remains temporarily for deployed-version
compatibility. Remove it only in a later migration after confirming no active
mobile/backend version uses phone OTP.
